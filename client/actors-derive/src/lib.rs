/*!
# StorageHub Actors Derive

This crate provides procedural macros to reduce boilerplate code in the StorageHub actors framework.

## Features

- `ActorEvent` derive macro: Implements `EventBusMessage` for event structs and registers them with a specific actor.
- `ActorEventBus` attribute macro: Generates the event bus provider struct and implements all the required methods and traits.

## Usage

First, add the dependency to your crate by including it in your Cargo.toml:

```toml
[dependencies]
shc-actors-derive = { workspace = true }
```

### 1. Defining Event Messages

Import the macros directly and use the `ActorEvent` derive macro:

```rust
use shc_actors_derive::ActorEvent;

#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct NewChallengeSeed {
    pub provider_id: String,
    pub tick: u32,
    pub seed: Vec<u8>,
}
```

### 2. Creating Event Bus Providers

Use the `ActorEventBus` attribute macro:

```rust
use shc_actors_derive::ActorEventBus;

#[ActorEventBus("blockchain_service")]
pub struct BlockchainServiceEventBusProvider;
```

## How It Works

1. The `ActorEvent` derive macro registers each event type with an actor ID in a global registry.
2. The `ActorEventBus` attribute macro looks up all the event types registered for the specified actor ID and generates the required code.

This approach greatly reduces boilerplate code while maintaining type safety and performance.

## Limitations

- All event types must be defined and processed before the `ActorEventBus` macro is used.
- The macro relies on a global state to keep track of registered events, which may cause issues in certain build scenarios.
*/

use once_cell::sync::Lazy;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{quote, ToTokens};
use std::sync::Mutex;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    spanned::Spanned,
    Attribute, DeriveInput, Ident, LitStr, Token,
};

/// Parser for the `#[actor(actor = "...")]` attribute that accompanies the `ActorEvent` derive macro.
///
/// # Usage
///
/// ```rust
/// #[derive(Debug, Clone, ActorEvent)]
/// #[actor(actor = "blockchain_service")]
/// pub struct MyEvent {
///     // fields...
/// }
/// ```
///
/// The `actor` parameter specifies which actor this event is registered with.
/// This is used by the `ActorEventBus` macro to automatically generate the appropriate
/// event bus implementations. All events with the same actor ID will be included in the
/// corresponding event bus provider.
#[allow(dead_code)]
struct ActorEventArgs {
    /// The actor ID string (e.g., "blockchain_service") that this event is associated with.
    /// This ID is used to register the event with a specific actor's event bus.
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

/// Parser for the `#[ActorEventBus("...")]` attribute macro.
///
/// # Usage
///
/// ```rust
/// #[ActorEventBus("blockchain_service")]
/// pub struct BlockchainServiceEventBusProvider;
/// ```
///
/// The string parameter is the actor ID that this event bus provider will handle.
/// The macro will automatically find all event types that were registered with this actor ID
/// using the `#[actor(actor = "...")]` attribute and generate the appropriate
/// fields and implementations.
struct ActorEventBusArgs {
    /// The actor ID string (e.g., "blockchain_service") for which this provider will handle events.
    /// All events registered with this ID will be included in the generated code.
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
static ACTOR_REGISTRY: Lazy<Mutex<ActorRegistry>> =
    Lazy::new(|| Mutex::new(ActorRegistry::default()));

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

/// A derive macro for implementing `EventBusMessage` for event structs.
///
/// This macro automatically:
/// - Implements `EventBusMessage` for the struct
/// - Registers the event with the specified actor ID
///
/// # Usage
///
/// ```rust
/// use shc_actors_derive::ActorEvent;
///
/// #[derive(Debug, Clone, ActorEvent)]
/// #[actor(actor = "blockchain_service")]
/// pub struct NewChallengeSeed {
///     pub provider_id: String,
///     pub tick: u32,
///     pub seed: Vec<u8>,
/// }
/// ```
///
/// # Attributes
///
/// - `#[actor(actor = "actor_id")]`: Required. Specifies which actor this event is registered with.
///   The `actor_id` is a string identifier for the actor.
#[proc_macro_derive(ActorEvent, attributes(actor))]
pub fn derive_actor_event(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    // Find the actor attribute
    let actor_attr = input
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("actor"));
    let actor_id = match actor_attr {
        Some(attr) => match get_actor_id_from_attr(attr) {
            Some(id) => id,
            None => {
                return syn::Error::new(
                        attr.span(),
                        "Failed to parse actor attribute: expected format #[actor(actor = \"actor_id\")]",
                    )
                    .to_compile_error()
                    .into();
            }
        },
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

/// An attribute macro for generating the event bus provider struct for an actor.
///
/// This macro automatically:
/// - Adds fields for each event bus registered with the specified actor ID
/// - Implements the `new()` method to initialize all event buses
/// - Implements `ProvidesEventBus<T>` for each event type
///
/// # Usage
///
/// ```rust
/// use shc_actors_derive::ActorEventBus;
///
/// #[ActorEventBus("blockchain_service")]
/// pub struct BlockchainServiceEventBusProvider;
/// ```
///
/// This will expand to include all the necessary fields and implementations for
/// every event that was registered with the "blockchain_service" actor ID using
/// the `ActorEvent` derive macro.
///
/// # Important Note
///
/// All event types must be defined and processed before this macro is used.
/// The order of declaration in your code matters.
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
