use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{quote, ToTokens};
use std::sync::Mutex;
use once_cell::sync::Lazy;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, Attribute, DeriveInput, Ident, LitStr, Token,
    spanned::Spanned,
};

/// Custom parser for ActorEvent attribute arguments
#[allow(dead_code)]
struct ActorEventArgs {
    actor: LitStr,
}

impl Parse for ActorEventArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // The format we're trying to parse is: actor = "actor_id"
        let name: Ident = input.parse()?;
        if name != "actor" {
            return Err(syn::Error::new(
                name.span(),
                "Expected `actor` as the attribute key",
            ));
        }
        
        let _: Token![=] = input.parse()?;
        let actor: LitStr = input.parse()?;
        
        Ok(ActorEventArgs { actor })
    }
}

/// Custom parser for ActorEventBus attribute arguments
struct ActorEventBusArgs {
    actor: LitStr,
}

impl Parse for ActorEventBusArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // Parse the actor ID from the attribute
        let lit_str = input.parse::<LitStr>()?;
        
        Ok(ActorEventBusArgs { actor: lit_str })
    }
}

/// A registry to store and generate code for actor event types
#[derive(Default)]
struct ActorRegistry {
    actors: std::collections::HashMap<String, Vec<String>>,
}

impl ActorRegistry {
    fn register_event(&mut self, actor_id: &str, event_type: &str) {
        self.actors
            .entry(actor_id.to_string())
            .or_default()
            .push(event_type.to_string());
    }
}

// Thread-safe registry using once_cell and Mutex
static ACTOR_REGISTRY: Lazy<Mutex<ActorRegistry>> = Lazy::new(|| {
    Mutex::new(ActorRegistry::default())
});

fn get_registry() -> std::sync::MutexGuard<'static, ActorRegistry> {
    ACTOR_REGISTRY.lock().unwrap()
}

// Extract actor ID from attribute using a simple approach with string operations
fn get_actor_id_from_attr(attr: &Attribute) -> Option<String> {
    // Convert the meta to a string and parse manually
    let meta_str = attr.meta.to_token_stream().to_string();
    
    // Expected format: actor(actor = "actor_id")
    if !meta_str.starts_with("actor") {
        return None;
    }
    
    // Check if it contains the parameters
    if !meta_str.contains('(') || !meta_str.contains(')') {
        return None;
    }
    
    // Extract the part between parentheses
    let start_idx = meta_str.find('(').unwrap() + 1;
    let end_idx = meta_str.rfind(')').unwrap();
    let params = &meta_str[start_idx..end_idx];
    
    // Extract the actor value
    if !params.contains("actor") || !params.contains('=') {
        return None;
    }
    
    // Find the quoted value
    let quote_start = params.find('"').unwrap() + 1;
    let quote_end = params.rfind('"').unwrap();
    let actor_id = &params[quote_start..quote_end];
    
    Some(actor_id.to_string())
}

/// Derive macro for implementing EventBusMessage for event structs
/// and registering them with a specific actor
#[proc_macro_derive(ActorEvent, attributes(actor))]
pub fn derive_actor_event(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    
    // Find the actor attribute
    let actor_attr = input.attrs.iter().find(|attr| attr.path().is_ident("actor"));
    let actor_id = match actor_attr {
        Some(attr) => {
            match get_actor_id_from_attr(attr) {
                Some(id) => id,
                None => {
                    return syn::Error::new(
                        attr.span(),
                        "Failed to parse actor attribute: expected format #[actor(actor = \"actor_id\")]",
                    )
                    .to_compile_error()
                    .into();
                }
            }
        }
        None => {
            return syn::Error::new(
                Span::call_site(),
                "Missing actor attribute. Use #[actor(actor=\"actor_id\")]",
            )
            .to_compile_error()
            .into();
        }
    };
    
    // Register this event with the actor
    get_registry().register_event(&actor_id, &name.to_string());
    
    // Generate the implementation of EventBusMessage
    let expanded = quote! {
        impl ::shc_actors_framework::event_bus::EventBusMessage for #name {}
    };
    
    TokenStream::from(expanded)
}

/// Derive macro for generating the EventBusProvider struct for an actor
#[allow(non_snake_case)]
#[proc_macro_attribute]
pub fn ActorEventBus(args: TokenStream, input: TokenStream) -> TokenStream {
    let actor_args = parse_macro_input!(args as ActorEventBusArgs);
    let actor_id = actor_args.actor.value();
    let input = parse_macro_input!(input as DeriveInput);
    let provider_name = &input.ident;
    
    // Get all events registered for this actor
    let registry = get_registry();
    let events = match registry.actors.get(&actor_id) {
        Some(events) => events,
        None => {
            return syn::Error::new(
                Span::call_site(),
                format!("No events registered for actor '{}'", actor_id),
            )
            .to_compile_error()
            .into();
        }
    };
    
    // Generate field declarations for each event bus
    let event_bus_fields = events.iter().map(|event| {
        let field_name = format!("{}_event_bus", to_snake_case(event));
        let field_name_ident = Ident::new(&field_name, Span::call_site());
        let event_type = Ident::new(event, Span::call_site());
        
        quote! {
            #field_name_ident: ::shc_actors_framework::event_bus::EventBus<#event_type>
        }
    });
    
    // Generate initialization for each event bus in new()
    let event_bus_inits = events.iter().map(|event| {
        let field_name = format!("{}_event_bus", to_snake_case(event));
        let field_name_ident = Ident::new(&field_name, Span::call_site());
        
        quote! {
            #field_name_ident: ::shc_actors_framework::event_bus::EventBus::new()
        }
    });
    
    // Generate ProvidesEventBus implementations for each event type
    let provides_event_bus_impls = events.iter().map(|event| {
        let event_type = Ident::new(event, Span::call_site());
        let field_name = format!("{}_event_bus", to_snake_case(event));
        let field_name_ident = Ident::new(&field_name, Span::call_site());
        
        quote! {
            impl ::shc_actors_framework::event_bus::ProvidesEventBus<#event_type> for #provider_name {
                fn event_bus(&self) -> &::shc_actors_framework::event_bus::EventBus<#event_type> {
                    &self.#field_name_ident
                }
            }
        }
    });
    
    // Generate the final expanded code
    let expanded = quote! {
        #[derive(Clone, Default)]
        pub struct #provider_name {
            #(#event_bus_fields),*
        }
        
        impl #provider_name {
            pub fn new() -> Self {
                Self {
                    #(#event_bus_inits),*
                }
            }
        }
        
        #(#provides_event_bus_impls)*
    };
    
    TokenStream::from(expanded)
}

/// Helper function to convert CamelCase to snake_case
fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap());
        } else {
            result.push(c);
        }
    }
    result
}
