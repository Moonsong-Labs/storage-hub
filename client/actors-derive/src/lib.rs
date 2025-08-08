/*!
# StorageHub Actors Derive

This crate provides procedural macros to reduce boilerplate code in the StorageHub actors framework.

## Features

- `ActorEvent` derive macro: Implements `EventBusMessage` for event structs and registers them with a specific actor.
- `ActorEventBus` attribute macro: Generates the event bus provider struct and implements all the required methods and traits.
- `subscribe_actor_event` macro: Creates and starts an event bus listener for a specific event type and task.
- `subscribe_actor_event_map` macro: Simplifies subscribing multiple events to tasks with shared parameters.

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

```ignore
use shc_actors_derive::ActorEventBus;

#[ActorEventBus("blockchain_service")]
pub struct BlockchainServiceEventBusProvider;
```

### 3. Subscribing to Events

For subscribing to a single event:

```ignore
subscribe_actor_event!(
    event: NewChallengeSeed,
    task: SubmitProofTask,
    service: &self.blockchain,
    spawner: &self.task_spawner,
    context: self.clone(),
    critical: true,
);
```

For subscribing to multiple events with shared parameters:

```ignore
subscribe_actor_event_map!(
    service: &self.blockchain,
    spawner: &self.task_spawner,
    context: self.clone(),
    critical: true,
    [
        NewStorageRequest => { task: MspUploadFileTask, critical: false },
        ProcessMspRespondStoringRequest => MspUploadFileTask,
        FinalisedMspStoppedStoringBucket => MspDeleteBucketTask,
    ]
);
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
    Attribute, DeriveInput, Ident, LitStr, Token, Type,
};

/// Parser for the `#[actor(actor = "...")]` attribute that accompanies the `ActorEvent` derive macro.
///
/// # Usage
///
/// ```ignore
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
/// ```ignore
/// #[ActorEventBus("blockchain_service")]
/// pub struct BlockchainServiceEventBusProvider;
/// ```
///
/// Or with generics:
///
/// ```ignore
/// #[ActorEventBus("blockchain_service", generics(Runtime: StorageEnableRuntime))]
/// pub struct BlockchainServiceEventBusProvider;
/// ```
///
/// The string parameter is the actor ID that this event bus provider will handle.
/// The optional generics parameter specifies generic type parameters and their bounds
/// for the generated provider struct.
struct ActorEventBusArgs {
    /// The actor ID string (e.g., "blockchain_service") for which this provider will handle events.
    /// All events registered with this ID will be included in the generated code.
    actor: LitStr,
    /// Optional generic type parameters and their bounds for the provider struct.
    /// These will be applied to the generated struct and all its implementations.
    generics: Vec<syn::WherePredicate>,
}

impl Parse for ActorEventBusArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // Parse the actor ID from the attribute
        let actor = input.parse::<LitStr>()?;
        let mut generics = Vec::new();

        // Check if there are additional parameters
        if input.peek(Token![,]) {
            let _: Token![,] = input.parse()?;

            while !input.is_empty() {
                let key: Ident = input.parse()?;

                if key == "generics" {
                    // Parse generics(T: SomeTrait, U: AnotherTrait)
                    let content;
                    let _ = syn::parenthesized!(content in input);

                    while !content.is_empty() {
                        let predicate: syn::WherePredicate = content.parse()?;
                        generics.push(predicate);

                        // Parse comma if there are more predicates
                        if content.peek(Token![,]) {
                            let _: Token![,] = content.parse()?;
                        }
                    }
                } else {
                    return Err(syn::Error::new(
                        key.span(),
                        format!("Unknown parameter: {}", key),
                    ));
                }

                // Parse comma if there are more fields
                if input.peek(Token![,]) {
                    let _: Token![,] = input.parse()?;
                }
            }
        }

        Ok(ActorEventBusArgs { actor, generics })
    }
}

/// Arguments for the subscribe_actor_event macro with named parameters
struct SubscribeActorEventArgs {
    task_type: syn::Type,
    event_type: syn::Type,
    service: syn::Expr,
    task_spawner: syn::Expr,
    context: Option<syn::Expr>,
    critical: Option<syn::LitBool>,
    task_instance: Option<syn::Expr>,
}

impl Default for SubscribeActorEventArgs {
    fn default() -> Self {
        Self {
            task_type: syn::parse_str("()").unwrap(),
            event_type: syn::parse_str("()").unwrap(),
            service: syn::parse_str("()").unwrap(),
            task_spawner: syn::parse_str("()").unwrap(),
            context: None,
            critical: None,
            task_instance: None,
        }
    }
}

impl Parse for SubscribeActorEventArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut args = SubscribeActorEventArgs::default();

        // Parse all named parameters
        while !input.is_empty() {
            let key: Ident = input.parse()?;
            let _: Token![:] = input.parse()?;

            if key == "task" {
                // This can either be a type or a task instance
                if input.peek(syn::token::Paren)
                    || input.peek(syn::token::Bracket)
                    || input.peek(syn::token::Brace)
                {
                    // Looks like an expression (task instance)
                    args.task_instance = Some(input.parse()?);
                } else {
                    // Assume it's a type
                    args.task_type = input.parse()?;
                }
            } else if key == "event" {
                args.event_type = input.parse()?;
            } else if key == "service" {
                args.service = input.parse()?;
            } else if key == "spawner" {
                args.task_spawner = input.parse()?;
            } else if key == "context" {
                args.context = Some(input.parse()?);
            } else if key == "critical" {
                args.critical = Some(input.parse()?);
            } else {
                return Err(syn::Error::new(
                    key.span(),
                    format!("Unknown parameter: {}", key),
                ));
            }

            // Parse comma if there are more fields
            if input.peek(Token![,]) {
                let _: Token![,] = input.parse()?;
            }
        }

        // Validate required fields
        if args.event_type.to_token_stream().to_string() == "()" {
            return Err(syn::Error::new(
                Span::call_site(),
                "Missing required parameter 'event'",
            ));
        }

        if args.task_spawner.to_token_stream().to_string() == "()" {
            return Err(syn::Error::new(
                Span::call_site(),
                "Missing required parameter 'spawner'",
            ));
        }

        if args.service.to_token_stream().to_string() == "()" {
            return Err(syn::Error::new(
                Span::call_site(),
                "Missing required parameter 'service'",
            ));
        }

        if args.task_type.to_token_stream().to_string() == "()" && args.task_instance.is_none() {
            return Err(syn::Error::new(
                Span::call_site(),
                "Missing required parameter 'task'",
            ));
        }

        if args.task_instance.is_some() && args.context.is_some() {
            return Err(syn::Error::new(
                Span::call_site(),
                "Cannot specify both 'task' as an instance and 'context'. 'context' is only valid when 'task' is a type.",
            ));
        }

        if args.task_type.to_token_stream().to_string() != "()"
            && args.context.is_none()
            && args.task_instance.is_none()
        {
            return Err(syn::Error::new(
                Span::call_site(),
                "When 'task' is a type, 'context' is required to create a new task instance.",
            ));
        }

        Ok(args)
    }
}

/// Information about an event type including its generics
#[derive(Clone)]
struct EventTypeInfo {
    name: String,
    generics: String, // The type generics as string (e.g., "<Runtime>")
    where_clause: String, // The where clause as string if any
}

/// A registry to store and generate code for actor event types
#[derive(Default)]
struct ActorRegistry {
    actors: std::collections::HashMap<String, Vec<EventTypeInfo>>,
}

impl ActorRegistry {
    fn register_event_with_generics(
        &mut self, 
        actor_id: &str, 
        event_type: &str,
        generics: String,
        where_clause: String,
    ) {
        let event_info = EventTypeInfo {
            name: event_type.to_string(),
            generics,
            where_clause,
        };
        self.actors
            .entry(actor_id.to_string())
            .or_default()
            .push(event_info);
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

    // Generate the implementation of EventBusMessage with proper generic support
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    // Register this event with the actor, including generic information
    get_registry().register_event_with_generics(
        &actor_id, 
        &name.to_string(),
        ty_generics.to_token_stream().to_string(),
        where_clause.to_token_stream().to_string(),
    );

    let expanded = quote! {
        impl #impl_generics ::shc_actors_framework::event_bus::EventBusMessage for #name #ty_generics #where_clause {}
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
/// ```ignore
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
    let user_generics = &actor_args.generics;
    let input = parse_macro_input!(input as DeriveInput);
    let provider_name = &input.ident;

    // Generate generic parameters and where clause from user-provided generics
    let (provider_generics, provider_where_clause) = 
        generate_provider_generics_for_declaration(user_generics);

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
    let event_bus_fields = events.iter().map(|event_info| {
        let field_name = format!("{}_event_bus", to_snake_case(&event_info.name));
        let field_name_ident = Ident::new(&field_name, Span::call_site());
        let event_type = Ident::new(&event_info.name, Span::call_site());
        
        // Parse the generics string back to tokens if not empty
        let event_generics = if event_info.generics.is_empty() {
            quote! {}
        } else {
            match syn::parse_str::<proc_macro2::TokenStream>(&event_info.generics) {
                Ok(tokens) => tokens,
                Err(_) => quote! {}, // Fallback to no generics if parsing fails
            }
        };

        quote! {
            #field_name_ident: ::shc_actors_framework::event_bus::EventBus<#event_type #event_generics>
        }
    });

    // Generate initialization for each event bus in new()
    let event_bus_inits = events.iter().map(|event_info| {
        let field_name = format!("{}_event_bus", to_snake_case(&event_info.name));
        let field_name_ident = Ident::new(&field_name, Span::call_site());

        quote! {
            #field_name_ident: ::shc_actors_framework::event_bus::EventBus::new()
        }
    });

    // Generate ProvidesEventBus implementations for each event type
    let provides_event_bus_impls = events.iter().map(|event_info| {
        let event_type = Ident::new(&event_info.name, Span::call_site());
        let field_name = format!("{}_event_bus", to_snake_case(&event_info.name));
        let field_name_ident = Ident::new(&field_name, Span::call_site());
        
        // Parse the generics string back to tokens if not empty
        let event_generics = if event_info.generics.is_empty() {
            quote! {}
        } else {
            match syn::parse_str::<proc_macro2::TokenStream>(&event_info.generics) {
                Ok(tokens) => tokens,
                Err(_) => quote! {}, // Fallback to no generics if parsing fails
            }
        };

        // Parse the where clause string back to tokens if not empty
        let where_clause = if event_info.where_clause.is_empty() {
            quote! {}
        } else {
            match syn::parse_str::<proc_macro2::TokenStream>(&event_info.where_clause) {
                Ok(tokens) => tokens,
                Err(_) => quote! {}, // Fallback to no where clause if parsing fails
            }
        };

        quote! {
            impl #provider_generics ::shc_actors_framework::event_bus::ProvidesEventBus<#event_type #event_generics> for #provider_name #provider_generics #where_clause #provider_where_clause {
                fn event_bus(&self) -> &::shc_actors_framework::event_bus::EventBus<#event_type #event_generics> {
                    &self.#field_name_ident
                }
            }
        }
    });

    // Generate the final expanded code
    // Only derive Default if there are no generics (backwards compatibility)
    let derives = if user_generics.is_empty() {
        quote! { #[derive(Clone, Default)] }
    } else {
        quote! { #[derive(Clone)] }
    };

    let expanded = quote! {
        #derives
        pub struct #provider_name #provider_generics #provider_where_clause {
            #(#event_bus_fields),*
        }

        impl #provider_generics #provider_name #provider_generics #provider_where_clause {
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

/// A macro to simplify event subscription code with named parameters for better readability.
///
/// This macro creates and starts an event bus listener for the specified event type and task.
///
/// # Parameters
///
/// - `event`: The event type to subscribe to (required)
/// - `task`: Either a task type (if creating a new task) or a task instance (required)
/// - `service`: The service that provides the event bus (required)
/// - `spawner`: The task spawner for spawning the event handler (required)
/// - `context`: The context to create a new task (required when `task` is a type)
/// - `critical`: Whether the event is critical (optional, defaults to false)
///
/// # Examples
///
/// ```ignore
/// // Basic usage with task type and context
/// subscribe_actor_event!(
///     event: FinalisedBspConfirmStoppedStoring,
///     task: BspDeleteFileTask,
///     service: &self.blockchain,
///     spawner: &self.task_spawner,
///     context: self,
///     critical: true,
/// );
///
/// // With an existing task instance
/// let task = BspDeleteFileTask::new(self.clone());
/// subscribe_actor_event!(
///     event: FinalisedBspConfirmStoppedStoring,
///     task: task.clone(),
///     service: &self.blockchain,
///     spawner: &self.task_spawner,
///     critical: true,
/// );
/// ```
#[proc_macro]
pub fn subscribe_actor_event(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as SubscribeActorEventArgs);

    let event_type = args.event_type;
    let task_spawner = args.task_spawner;
    let service = args.service;

    // Generate a unique variable name based on the event type
    let type_str = event_type.to_token_stream().to_string();
    let var_name = format!("{}_event_bus_listener", to_snake_case(&type_str));
    let var_ident = syn::Ident::new(&var_name, Span::call_site());

    // Determine if the event is critical
    let critical = args.critical.map_or(false, |lit| lit.value);
    let critical_lit = syn::LitBool::new(critical, Span::call_site());

    // If a task instance is provided, use it
    // Otherwise, create a new task using the task type and context
    let result = if let Some(task) = args.task_instance {
        quote! {
            let #var_ident: ::shc_actors_framework::event_bus::EventBusListener<#event_type, _> =
                #task.subscribe_to(#task_spawner, #service, #critical_lit);
            #var_ident.start();
        }
    } else {
        let task_type = args.task_type;

        if let Some(context) = args.context {
            quote! {
                let task = #task_type::new(#context.clone());
                let #var_ident: ::shc_actors_framework::event_bus::EventBusListener<#event_type, _> =
                    task.subscribe_to(#task_spawner, #service, #critical_lit);
                #var_ident.start();
            }
        } else {
            // This shouldn't happen due to validation in the Parse implementation
            syn::Error::new(
                Span::call_site(),
                "Internal error: Task type provided without context",
            )
            .to_compile_error()
        }
    };

    result.into()
}

/// A macro to simplify mapping multiple events to tasks with shared parameters.
///
/// This macro calls `subscribe_actor_event!` for each event-task pair, applying common parameters
/// and allowing for per-mapping overrides.
///
/// # Parameters
///
/// - `service`: The service that provides the event bus (required)
/// - `spawner`: The task spawner for spawning event handlers (required)
/// - `context`: The context to create new tasks (required)
/// - `critical`: Default critical value for all mappings (optional, defaults to false)
/// - An array of mappings, where each mapping is either:
///   - `EventType => TaskType`: Uses default critical value
///   - `EventType => { task: TaskType, critical: bool }`: Overrides critical value for this mapping
///
/// # Examples
///
/// ```ignore
/// // Basic usage with multiple event-task mappings
/// subscribe_actor_event_map!(
///     service: &self.blockchain,
///     spawner: &self.task_spawner,
///     context: self.clone(),
///     critical: true,
///     [
///         // Override critical for specific mapping
///         NewStorageRequest => { task: MspUploadFileTask, critical: false },
///         // Use default critical value
///         ProcessMspRespondStoringRequest => MspUploadFileTask,
///         FinalisedMspStoppedStoringBucket => MspDeleteBucketTask,
///     ]
/// );
/// ```
#[proc_macro]
pub fn subscribe_actor_event_map(input: TokenStream) -> TokenStream {
    struct ActorEventMapArgs {
        service: syn::Expr,
        spawner: syn::Expr,
        context: syn::Expr,
        critical: Option<syn::LitBool>,
        mappings: Vec<EventTaskMapping>,
    }

    struct EventTaskMapping {
        event_type: syn::Type,
        task_type: syn::Type,
        critical_override: Option<syn::LitBool>,
    }

    impl Parse for ActorEventMapArgs {
        fn parse(input: ParseStream) -> syn::Result<Self> {
            let mut service = None;
            let mut spawner = None;
            let mut context = None;
            let mut critical = None;
            let mut mappings = Vec::new();

            // Parse named parameters
            while !input.peek(syn::token::Bracket) {
                let key: Ident = input.parse()?;
                let _: Token![:] = input.parse()?;

                if key == "service" {
                    service = Some(input.parse()?);
                } else if key == "spawner" {
                    spawner = Some(input.parse()?);
                } else if key == "context" {
                    context = Some(input.parse()?);
                } else if key == "critical" {
                    critical = Some(input.parse()?);
                } else {
                    return Err(syn::Error::new(
                        key.span(),
                        format!("Unknown parameter: {}", key),
                    ));
                }

                // Parse comma if there are more fields
                if input.peek(Token![,]) {
                    let _: Token![,] = input.parse()?;
                }
            }

            // Parse mappings array
            let content;
            let _ = syn::bracketed!(content in input);

            while !content.is_empty() {
                // Parse event type
                let event_type: syn::Type = content.parse()?;

                // Parse =>
                let _: Token![=>] = content.parse()?;

                // Parse task type or task config
                let (task_type, critical_override) = if content.peek(syn::token::Brace) {
                    // Complex form: EventType => { task: TaskType, critical: bool }
                    let inner_content;
                    let _ = syn::braced!(inner_content in content);

                    let mut task = None;
                    let mut crit_override = None;

                    while !inner_content.is_empty() {
                        let key: Ident = inner_content.parse()?;
                        let _: Token![:] = inner_content.parse()?;

                        if key == "task" {
                            task = Some(inner_content.parse()?);
                        } else if key == "critical" {
                            crit_override = Some(inner_content.parse()?);
                        } else {
                            return Err(syn::Error::new(
                                key.span(),
                                format!("Unknown mapping parameter: {}", key),
                            ));
                        }

                        // Parse comma if there are more fields
                        if inner_content.peek(Token![,]) {
                            let _: Token![,] = inner_content.parse()?;
                        }
                    }

                    (
                        task.ok_or_else(|| {
                            syn::Error::new(
                                Span::call_site(),
                                "Missing required parameter 'task' in mapping",
                            )
                        })?,
                        crit_override,
                    )
                } else {
                    // Simple form: EventType => TaskType
                    (content.parse()?, None)
                };

                mappings.push(EventTaskMapping {
                    event_type,
                    task_type,
                    critical_override,
                });

                // Parse comma if there are more mappings
                if content.peek(Token![,]) {
                    let _: Token![,] = content.parse()?;
                }
            }

            // Validate required fields
            let service = service.ok_or_else(|| {
                syn::Error::new(Span::call_site(), "Missing required parameter 'service'")
            })?;

            let spawner = spawner.ok_or_else(|| {
                syn::Error::new(Span::call_site(), "Missing required parameter 'spawner'")
            })?;

            let context = context.ok_or_else(|| {
                syn::Error::new(Span::call_site(), "Missing required parameter 'context'")
            })?;

            Ok(ActorEventMapArgs {
                service,
                spawner,
                context,
                critical,
                mappings,
            })
        }
    }

    let args = parse_macro_input!(input as ActorEventMapArgs);

    // Default critical value
    let default_critical = args
        .critical
        .map_or_else(|| syn::LitBool::new(false, Span::call_site()), |c| c);

    // Generate subscribe_actor_event! calls for each mapping
    let calls = args.mappings.iter().map(|mapping| {
        let event_type = &mapping.event_type;
        let task_type = &mapping.task_type;
        let service = &args.service;
        let spawner = &args.spawner;
        let context = &args.context;

        // Use mapping-specific critical value if provided, otherwise use default
        let critical = mapping
            .critical_override
            .as_ref()
            .unwrap_or(&default_critical);

        quote! {
            subscribe_actor_event!(
                event: #event_type,
                task: #task_type,
                service: #service,
                spawner: #spawner,
                context: #context,
                critical: #critical,
            );
        }
    });

    // Combine all calls
    let expanded = quote! {
        #(#calls)*
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

/// Generate generics for struct declarations from additional generics
fn generate_provider_generics_for_declaration(
    additional_generics: &[syn::WherePredicate],
) -> (proc_macro2::TokenStream, proc_macro2::TokenStream) {
    if additional_generics.is_empty() {
        return (quote! {}, quote! {});
    }

    // Extract type parameter names for the generic parameter list
    let mut type_params = Vec::new();
    let mut where_predicates = Vec::new();

    for predicate in additional_generics {
        if let syn::WherePredicate::Type(type_pred) = predicate {
            if let syn::Type::Path(type_path) = &type_pred.bounded_ty {
                if let Some(first_segment) = type_path.path.segments.first() {
                    type_params.push(&first_segment.ident);
                }
            }
        }
        where_predicates.push(quote!(#predicate));
    }

    let generics_params = if !type_params.is_empty() {
        quote! { <#(#type_params),*> }
    } else {
        quote! {}
    };

    let where_clause = if !where_predicates.is_empty() {
        quote! { where #(#where_predicates),* }
    } else {
        quote! {}
    };

    (generics_params, where_clause)
}

/// Generate generics for enum and trait declarations from additional generics
fn generate_additional_generics_for_declaration(
    additional_generics: &[syn::WherePredicate],
) -> (proc_macro2::TokenStream, proc_macro2::TokenStream) {
    if additional_generics.is_empty() {
        return (quote! {}, quote! {});
    }

    let param_names: Vec<syn::Ident> = additional_generics
        .iter()
        .filter_map(|pred| {
            if let syn::WherePredicate::Type(type_pred) = pred {
                if let syn::Type::Path(type_path) = &type_pred.bounded_ty {
                    if let Some(first_segment) = type_path.path.segments.first() {
                        return Some(first_segment.ident.clone());
                    }
                }
            }
            None
        })
        .collect();

    let generics_params = quote! { <#(#param_names),*> };
    let where_clause = quote! { where #(#additional_generics),* };

    (generics_params, where_clause)
}

/// Merge and deduplicate generics from service type and additional generics
fn merge_and_deduplicate_generics(
    service_generics: &[syn::Ident],
    service_bounds: &[proc_macro2::TokenStream],
    additional_generics: &[syn::WherePredicate],
) -> (proc_macro2::TokenStream, proc_macro2::TokenStream) {
    let service_param_names: std::collections::HashSet<String> = service_generics
        .iter()
        .map(|ident| ident.to_string())
        .collect();

    // Filter additional generics to exclude duplicates
    let mut filtered_additional_params = Vec::new();
    let mut filtered_additional_bounds = Vec::new();

    for predicate in additional_generics {
        if let syn::WherePredicate::Type(type_pred) = predicate {
            if let syn::Type::Path(type_path) = &type_pred.bounded_ty {
                if let Some(first_segment) = type_path.path.segments.first() {
                    let param_name = first_segment.ident.to_string();
                    if !service_param_names.contains(&param_name) {
                        filtered_additional_params.push(first_segment.ident.clone());
                        filtered_additional_bounds.push(quote!(#predicate));
                    }
                }
            }
        }
    }

    // Combine all parameters and bounds
    let all_params: Vec<syn::Ident> = service_generics
        .iter()
        .cloned()
        .chain(filtered_additional_params.into_iter())
        .collect();

    let all_bounds: Vec<proc_macro2::TokenStream> = service_bounds
        .iter()
        .cloned()
        .chain(filtered_additional_bounds.into_iter())
        .collect();

    let impl_generics = if !all_params.is_empty() {
        quote! { <#(#all_params),*> }
    } else {
        quote! {}
    };

    let where_clause = if !all_bounds.is_empty() {
        quote! { where #(#all_bounds),* }
    } else {
        quote! {}
    };

    (impl_generics, where_clause)
}

/// Parser for the `#[actor_command(...)]` attribute macro
///
/// # Usage
///
/// ```ignore
/// #[actor_command(
///     service = ServiceType,
///     default_mode = "ImmediateResponse",
///     default_error_type = CustomError,
///     default_inner_channel_type = "futures::channel::oneshot::Receiver",
///     generics(T: SomeTrait, U: AnotherTrait)
/// )]
/// pub enum CommandEnum {
///     // command variants
/// }
/// ```
struct ActorCommandArgs {
    service: Type,
    default_mode: String,
    default_error_type: Option<Type>,
    default_inner_channel_type: Option<Type>,
    additional_generics: Vec<syn::WherePredicate>,
}

impl Parse for ActorCommandArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut service = None;
        let mut default_mode = String::from("ImmediateResponse");
        let mut default_error_type = None;
        let mut default_inner_channel_type = None;
        let mut additional_generics = Vec::new();

        while !input.is_empty() {
            let key: Ident = input.parse()?;

            if key == "generics" {
                // Parse generics(T: SomeTrait, U: AnotherTrait)
                let content;
                let _ = syn::parenthesized!(content in input);

                while !content.is_empty() {
                    let predicate: syn::WherePredicate = content.parse()?;
                    additional_generics.push(predicate);

                    // Parse comma if there are more predicates
                    if content.peek(Token![,]) {
                        let _: Token![,] = content.parse()?;
                    }
                }
            } else {
                let _: Token![=] = input.parse()?;

                if key == "service" {
                    // Parse the service type with its bounds
                    let service_type: Type = input.parse()?;
                    service = Some(service_type);
                } else if key == "default_mode" {
                    let mode: LitStr = input.parse()?;
                    default_mode = mode.value();
                } else if key == "default_error_type" {
                    default_error_type = Some(input.parse()?);
                } else if key == "default_inner_channel_type" {
                    default_inner_channel_type = Some(input.parse()?);
                } else {
                    return Err(syn::Error::new(
                        key.span(),
                        format!("Unknown parameter: {}", key),
                    ));
                }
            }

            // Parse comma if there are more fields
            if input.peek(Token![,]) {
                let _: Token![,] = input.parse()?;
            }
        }

        let service = service.ok_or_else(|| {
            syn::Error::new(Span::call_site(), "Missing required parameter 'service'")
        })?;

        Ok(ActorCommandArgs {
            service,
            default_mode,
            default_error_type,
            default_inner_channel_type,
            additional_generics,
        })
    }
}

/// Parser for the `#[command(...)]` attribute
///
/// # Usage
///
/// ```ignore
/// #[command(
///     mode = "ImmediateResponse",
///     success_type = SomeType,
///     error_type = CustomError,
///     inner_channel_type = "futures::channel::oneshot::Receiver"
/// )]
/// CommandVariant { ... }
/// ```
struct CommandVariantArgs {
    mode: Option<String>,
    success_type: Option<Type>,
    error_type: Option<Type>,
    inner_channel_type: Option<Type>,
}

impl Parse for CommandVariantArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut mode = None;
        let mut success_type = None;
        let mut error_type = None;
        let mut inner_channel_type = None;

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            let _: Token![=] = input.parse()?;

            if key == "mode" {
                let mode_lit: LitStr = input.parse()?;
                mode = Some(mode_lit.value());
            } else if key == "success_type" {
                success_type = Some(input.parse()?);
            } else if key == "error_type" {
                error_type = Some(input.parse()?);
            } else if key == "inner_channel_type" {
                inner_channel_type = Some(input.parse()?);
            } else {
                return Err(syn::Error::new(
                    key.span(),
                    format!("Unknown parameter: {}", key),
                ));
            }

            // Parse comma if there are more fields
            if input.peek(Token![,]) {
                let _: Token![,] = input.parse()?;
            }
        }

        Ok(CommandVariantArgs {
            mode,
            success_type,
            error_type,
            inner_channel_type,
        })
    }
}

/// Parse attributes to find and extract the command variant args
fn extract_command_variant_args(attrs: &[Attribute]) -> Option<CommandVariantArgs> {
    for attr in attrs {
        if attr.path().is_ident("command") {
            return match attr.parse_args() {
                Ok(args) => Some(args),
                Err(_) => None,
            };
        }
    }
    None
}

/// Generate callback field and type for a command variant based on mode and return types
fn generate_callback_type(
    mode: &str,
    success_type: &Option<Type>,
    error_type: &Option<Type>,
    default_error_type: &Option<Type>,
    inner_channel_type: &Option<Type>,
) -> proc_macro2::TokenStream {
    match mode {
        "FireAndForget" => {
            // No callback needed for fire and forget
            quote! {}
        }
        "ImmediateResponse" => {
            let success_type = success_type
                .clone()
                .unwrap_or_else(|| syn::parse_str("()").expect("Failed to parse unit type"));

            let error_type = error_type
                .clone()
                .or_else(|| default_error_type.clone())
                .unwrap_or_else(|| {
                    syn::parse_str("anyhow::Error").expect("Failed to parse default error type")
                });

            quote! {
                callback: tokio::sync::oneshot::Sender<Result<#success_type, #error_type>>
            }
        }
        "AsyncResponse" => {
            let success_type = success_type
                .clone()
                .unwrap_or_else(|| syn::parse_str("()").expect("Failed to parse unit type"));

            let error_type = error_type.clone().unwrap_or_else(|| {
                syn::parse_str("anyhow::Error").expect("Failed to parse default error type")
            });

            // Use custom channel type if provided
            if let Some(channel_type) = inner_channel_type {
                quote! {
                    callback: tokio::sync::oneshot::Sender<#channel_type<Result<#success_type, #error_type>>>
                }
            } else {
                quote! {
                    callback: tokio::sync::oneshot::Sender<
                        futures::channel::oneshot::Receiver<Result<#success_type, #error_type>>
                    >
                }
            }
        }
        _ => panic!("Invalid mode: {}", mode),
    }
}

/// Generate method parameters from command variant fields
fn generate_method_params(fields: &syn::Fields) -> Vec<proc_macro2::TokenStream> {
    let mut params = Vec::new();

    match fields {
        syn::Fields::Named(fields_named) => {
            for field in &fields_named.named {
                if let Some(name) = &field.ident {
                    if name != "callback" {
                        let ty = &field.ty;
                        params.push(quote! { #name: #ty });
                    }
                }
            }
        }
        syn::Fields::Unnamed(fields_unnamed) => {
            for (i, field) in fields_unnamed.unnamed.iter().enumerate() {
                let name = Ident::new(&format!("arg{}", i), Span::call_site());
                let ty = &field.ty;
                params.push(quote! { #name: #ty });
            }
        }
        syn::Fields::Unit => {}
    }

    params
}

/// Generate method arguments to be used in command construction
fn generate_method_args(fields: &syn::Fields) -> Vec<proc_macro2::TokenStream> {
    let mut args = Vec::new();

    match fields {
        syn::Fields::Named(fields_named) => {
            for field in &fields_named.named {
                if let Some(name) = &field.ident {
                    if name != "callback" {
                        args.push(quote! { #name });
                    }
                }
            }
        }
        syn::Fields::Unnamed(fields_unnamed) => {
            for i in 0..fields_unnamed.unnamed.len() {
                let name = Ident::new(&format!("arg{}", i), Span::call_site());
                args.push(quote! { #name });
            }
        }
        syn::Fields::Unit => {}
    }

    args
}

/// Generate method implementation for a command variant
fn generate_method_impl(
    enum_name: &Ident,
    variant_name: &Ident,
    fields: &syn::Fields,
    mode: &str,
    success_type: &Option<Type>,
    error_type: &Option<Type>,
    inner_channel_type: &Option<Type>,
) -> (proc_macro2::TokenStream, proc_macro2::TokenStream) {
    let method_name = Ident::new(&to_snake_case(&variant_name.to_string()), Span::call_site());
    let params = generate_method_params(fields);
    let args = generate_method_args(fields);

    // Determine if this is a unit variant with a callback field
    let is_unit_variant_with_callback =
        matches!(fields, syn::Fields::Unit) && mode != "FireAndForget";

    // Method signature for the trait (no implementation)
    let method_signature = match mode {
        "FireAndForget" => {
            quote! {
                async fn #method_name(&self, #(#params),*);
            }
        }
        "ImmediateResponse" => {
            let success_type = success_type
                .clone()
                .unwrap_or_else(|| syn::parse_str("()").expect("Failed to parse unit type"));

            let error_type = error_type.clone().unwrap_or_else(|| {
                syn::parse_str("anyhow::Error").expect("Failed to parse default error type")
            });

            quote! {
                async fn #method_name(&self, #(#params),*) -> Result<#success_type, #error_type>;
            }
        }
        "AsyncResponse" => {
            let success_type = success_type
                .clone()
                .unwrap_or_else(|| syn::parse_str("()").expect("Failed to parse unit type"));

            let error_type = error_type.clone().unwrap_or_else(|| {
                syn::parse_str("anyhow::Error").expect("Failed to parse default error type")
            });

            quote! {
                async fn #method_name(&self, #(#params),*) -> Result<#success_type, #error_type>;
            }
        }
        _ => panic!("Invalid mode: {}", mode),
    };

    // Method implementation for the impl block
    let method_impl = match mode {
        "FireAndForget" => {
            if matches!(fields, syn::Fields::Unit) {
                quote! {
                    async fn #method_name(&self, #(#params),*) {
                        let command = #enum_name::#variant_name;
                        self.send(command).await;
                    }
                }
            } else {
                quote! {
                    async fn #method_name(&self, #(#params),*) {
                        let command = #enum_name::#variant_name {
                            #(#args),*
                        };
                        self.send(command).await;
                    }
                }
            }
        }
        "ImmediateResponse" => {
            let success_type = success_type
                .clone()
                .unwrap_or_else(|| syn::parse_str("()").expect("Failed to parse unit type"));

            let error_type = error_type.clone().unwrap_or_else(|| {
                syn::parse_str("anyhow::Error").expect("Failed to parse default error type")
            });

            if is_unit_variant_with_callback {
                quote! {
                    async fn #method_name(&self, #(#params),*) -> Result<#success_type, #error_type> {
                        let (callback, rx) = tokio::sync::oneshot::channel();
                        let command = #enum_name::#variant_name {
                            callback,
                        };
                        self.send(command).await;
                        rx.await.expect("Failed to receive response from service. Probably means service has crashed.")
                    }
                }
            } else {
                quote! {
                    async fn #method_name(&self, #(#params),*) -> Result<#success_type, #error_type> {
                        let (callback, rx) = tokio::sync::oneshot::channel();
                        let command = #enum_name::#variant_name {
                            #(#args),*,
                            callback,
                        };
                        self.send(command).await;
                        rx.await.expect("Failed to receive response from service. Probably means service has crashed.")
                    }
                }
            }
        }
        "AsyncResponse" => {
            let success_type = success_type
                .clone()
                .unwrap_or_else(|| syn::parse_str("()").expect("Failed to parse unit type"));

            let error_type = error_type.clone().unwrap_or_else(|| {
                syn::parse_str("anyhow::Error").expect("Failed to parse default error type")
            });

            // Use custom channel type if provided
            if let Some(channel_type) = inner_channel_type {
                if is_unit_variant_with_callback {
                    quote! {
                        async fn #method_name(&self, #(#params),*) -> Result<#success_type, #error_type> {
                            let (callback, service_rx) = tokio::sync::oneshot::channel();
                            let command = #enum_name::#variant_name {
                                callback,
                            };
                            self.send(command).await;

                            // Wait for the response from the service
                            let network_rx: #channel_type<Result<#success_type, #error_type>> = service_rx.await
                                .expect("Failed to receive response from service. Probably means service has crashed.");

                            // Now we wait on the actual response from the async operation
                            network_rx.await
                                .expect("Failed to receive response from the async operation. Probably means it has crashed.")
                        }
                    }
                } else {
                    quote! {
                        async fn #method_name(&self, #(#params),*) -> Result<#success_type, #error_type> {
                            let (callback, service_rx) = tokio::sync::oneshot::channel();
                            let command = #enum_name::#variant_name {
                                #(#args),*,
                                callback,
                            };
                            self.send(command).await;

                            // Wait for the response from the service
                            let network_rx: #channel_type<Result<#success_type, #error_type>> = service_rx.await
                                .expect("Failed to receive response from service. Probably means service has crashed.");

                            // Now we wait on the actual response from the async operation
                            network_rx.await
                                .expect("Failed to receive response from the async operation. Probably means it has crashed.")
                        }
                    }
                }
            } else {
                // Default implementation using futures::channel::oneshot::Receiver
                if is_unit_variant_with_callback {
                    quote! {
                        async fn #method_name(&self, #(#params),*) -> Result<#success_type, #error_type> {
                            let (callback, service_rx) = tokio::sync::oneshot::channel();
                            let command = #enum_name::#variant_name {
                                callback,
                            };
                            self.send(command).await;

                            // First we wait for the response from the service
                            let inner_rx = service_rx.await.expect("Failed to receive response from service. Probably means service has crashed.");

                            // Now we wait on the actual response from the async operation
                            inner_rx.await.expect("Failed to receive response from the async operation. Probably means it has crashed.")
                        }
                    }
                } else {
                    quote! {
                        async fn #method_name(&self, #(#params),*) -> Result<#success_type, #error_type> {
                            let (callback, service_rx) = tokio::sync::oneshot::channel();
                            let command = #enum_name::#variant_name {
                                #(#args),*,
                                callback,
                            };
                            self.send(command).await;

                            // First we wait for the response from the service
                            let inner_rx = service_rx.await.expect("Failed to receive response from service. Probably means service has crashed.");

                            // Now we wait on the actual response from the async operation
                            inner_rx.await.expect("Failed to receive response from the async operation. Probably means it has crashed.")
                        }
                    }
                }
            }
        }
        _ => panic!("Invalid mode: {}", mode),
    };

    (method_signature, method_impl)
}

/// Macro implementation for actor_command attribute
/// An attribute macro for generating an actor command interface for enum commands.
///
/// This macro automatically:
/// - Adds appropriate callback fields to each command variant based on the specified mode
/// - Generates an interface trait with methods for each command variant
/// - Implements the interface trait for ActorHandle<ServiceType>
///
/// # Parameters
///
/// - `service`: (Required) The service type that processes these commands
/// - `default_mode`: (Optional) Default command mode, one of: "FireAndForget", "ImmediateResponse", "AsyncResponse"
/// - `default_error_type`: (Optional) Default error type for command responses
/// - `default_inner_channel_type`: (Optional) Default channel type for AsyncResponse mode
/// - `generics`: (Optional) Additional generic parameters and their bounds to add to the enum and trait
///
/// # Command Mode Options
///
/// - `FireAndForget`: No response is expected
/// - `ImmediateResponse`: Wait for a direct response from the actor
/// - `AsyncResponse`: Wait for an asynchronous response (e.g., from a network operation)
///
/// # Usage
///
/// ```ignore
/// #[actor_command(
///     service = BlockchainService<FSH: ForestStorageHandler + Clone + Send + Sync + 'static>,
///     default_mode = "ImmediateResponse",
///     default_inner_channel_type = tokio::sync::oneshot::Receiver,
///     generics(Runtime: StorageEnableRuntime, OtherType: SomeTrait)
/// )]
/// pub enum BlockchainServiceCommand {
///     #[command(success_type = SubmittedTransaction)]
///     SendExtrinsic {
///         call: storage_hub_runtime::RuntimeCall,
///         options: SendExtrinsicOptions,
///     },
///     
///     #[command(success_type = Extrinsic)]
///     GetExtrinsicFromBlock {
///         block_hash: H256,
///         extrinsic_hash: H256,
///     },
///     
///     #[command(mode = "AsyncResponse", inner_channel_type = tokio::sync::oneshot::Receiver)]
///     WaitForBlock {
///         block_number: BlockNumber,
///     },
/// }
/// ```
///
/// This generates a trait `BlockchainServiceCommandInterface` with methods like `send_extrinsic`,
/// `get_extrinsic_from_block`, etc., which can be called on an `ActorHandle<BlockchainService>`.
///
/// # Command Variant Attributes
///
/// Each command variant can be annotated with a `#[command(...)]` attribute to specify its behaviour:
///
/// - `mode`: Override the default command mode
/// - `success_type`: The success type returned in the Result
/// - `error_type`: Override the default error type
/// - `inner_channel_type`: Override the default channel type for AsyncResponse mode
///
/// # Generated Interface
///
/// The generated interface trait will have method names in snake_case derived from the variant names.
/// For example, a variant named `SendExtrinsic` will generate a method named `send_extrinsic`.
#[proc_macro_attribute]
pub fn actor_command(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as ActorCommandArgs);
    let input = parse_macro_input!(input as syn::ItemEnum);

    let enum_name = &input.ident;
    let service_type = &args.service;
    let default_mode = args.default_mode;
    let default_error_type = args.default_error_type;
    let default_inner_channel_type = args.default_inner_channel_type;
    let additional_generics = &args.additional_generics;

    // Generate interface trait name
    let interface_name = Ident::new(&format!("{}Interface", enum_name), Span::call_site());

    // Generate additional generics for enum and trait declarations
    let (additional_generics_params, additional_where_clause) =
        generate_additional_generics_for_declaration(additional_generics);

    // Generate each variant with callback field if needed
    let mut updated_variants = Vec::new();
    let mut trait_method_signatures = Vec::new();
    let mut trait_method_implementations = Vec::new();

    for variant in &input.variants {
        let variant_name = &variant.ident;
        let variant_args = extract_command_variant_args(&variant.attrs);

        // Get mode and types
        let mode = variant_args
            .as_ref()
            .and_then(|args| args.mode.clone())
            .unwrap_or_else(|| default_mode.clone());

        let success_type = variant_args
            .as_ref()
            .and_then(|args| args.success_type.clone());
        let error_type = variant_args
            .as_ref()
            .and_then(|args| args.error_type.clone())
            .or_else(|| default_error_type.clone());

        let inner_channel_type = variant_args
            .as_ref()
            .and_then(|args| args.inner_channel_type.clone())
            .or_else(|| default_inner_channel_type.clone());

        // Generate callback field
        let callback_field = generate_callback_type(
            &mode,
            &success_type,
            &error_type,
            &default_error_type,
            &inner_channel_type,
        );

        // Add variant with callback field
        match &variant.fields {
            syn::Fields::Named(fields_named) => {
                let named_fields = fields_named.named.iter().map(|f| {
                    let name = &f.ident;
                    let ty = &f.ty;
                    quote! { #name: #ty }
                });

                let variant_quote = if callback_field.is_empty() {
                    quote! {
                        #variant_name {
                            #(#named_fields),*
                        }
                    }
                } else {
                    if fields_named.named.is_empty() {
                        quote! {
                            #variant_name {
                                #callback_field
                            }
                        }
                    } else {
                        quote! {
                            #variant_name {
                                #(#named_fields),*,
                                #callback_field
                            }
                        }
                    }
                };

                let mut final_variant_quote = variant_quote;
                for attr in &variant.attrs {
                    if !attr.path().is_ident("command") {
                        final_variant_quote = quote! {
                            #attr
                            #final_variant_quote
                        };
                    }
                }

                updated_variants.push(final_variant_quote);
            }
            syn::Fields::Unnamed(fields_unnamed) => {
                let unnamed_fields = fields_unnamed.unnamed.iter().map(|f| {
                    let ty = &f.ty;
                    quote! { #ty }
                });

                let mut variant_quote = quote! {
                    #variant_name(#(#unnamed_fields,)* #callback_field)
                };

                for attr in &variant.attrs {
                    if !attr.path().is_ident("command") {
                        variant_quote = quote! {
                            #attr
                            #variant_quote
                        };
                    }
                }

                updated_variants.push(variant_quote);
            }
            syn::Fields::Unit => {
                let mut variant_quote = if !callback_field.is_empty() {
                    quote! {
                        #variant_name {
                            #callback_field
                        }
                    }
                } else {
                    quote! {
                        #variant_name
                    }
                };

                for attr in &variant.attrs {
                    if !attr.path().is_ident("command") {
                        variant_quote = quote! {
                            #attr
                            #variant_quote
                        };
                    }
                }

                updated_variants.push(variant_quote);
            }
        }

        // Generate trait method signature and implementation
        let (method_signature, method_impl) = generate_method_impl(
            enum_name,
            variant_name,
            &variant.fields,
            &mode,
            &success_type,
            &error_type,
            &inner_channel_type,
        );
        trait_method_signatures.push(method_signature);
        trait_method_implementations.push(method_impl);
    }

    // Build the updated enum
    let vis = &input.vis;
    let generics = &input.generics;

    // Process the service type
    let (
        service_type_path,
        _service_impl_generics,
        _type_generics,
        _service_where_clause,
        service_generic_params,
        service_where_bounds,
    ) = process_service_type(service_type);

    // Merge and deduplicate generics for the impl block
    let (merged_impl_generics, merged_where_clause) = merge_and_deduplicate_generics(
        &service_generic_params,
        &service_where_bounds,
        additional_generics,
    );

    // Extract type parameter names for the trait implementation
    let additional_type_params: Vec<syn::Ident> = additional_generics
        .iter()
        .filter_map(|pred| {
            if let syn::WherePredicate::Type(type_pred) = pred {
                if let syn::Type::Path(type_path) = &type_pred.bounded_ty {
                    if let Some(first_segment) = type_path.path.segments.first() {
                        return Some(first_segment.ident.clone());
                    }
                }
            }
            None
        })
        .collect();

    let trait_type_params = if additional_type_params.is_empty() {
        quote! {}
    } else {
        quote! { <#(#additional_type_params),*> }
    };

    // Generate the interface trait and implementation
    let trait_def = quote! {
        #[async_trait::async_trait]
        pub trait #interface_name #additional_generics_params
        #additional_where_clause
        {
            #(#trait_method_signatures)*
        }

        #[async_trait::async_trait]
        impl #merged_impl_generics #interface_name #trait_type_params for shc_actors_framework::actor::ActorHandle<#service_type_path>
        #merged_where_clause
        {
            #(#trait_method_implementations)*
        }
    };

    // Output the result
    let result = quote! {
        #vis enum #enum_name #generics #additional_generics_params
        #additional_where_clause
        {
            #(#updated_variants,)*
        }

        #trait_def
    };

    result.into()
}

/// Process a service type to extract its path, generics, and where clause
fn process_service_type(
    service_type: &Type,
) -> (
    proc_macro2::TokenStream,
    proc_macro2::TokenStream,
    proc_macro2::TokenStream,
    proc_macro2::TokenStream,
    Vec<syn::Ident>,               // service generic parameters
    Vec<proc_macro2::TokenStream>, // service where bounds
) {
    // Default values for when we have no generics
    let default_result = (
        quote! { #service_type },
        quote! {},
        quote! {},
        quote! {},
        Vec::new(),
        Vec::new(),
    );

    // Parse the service type to extract its parts
    match service_type {
        Type::Path(type_path) if !type_path.path.segments.is_empty() => {
            // Get the last segment which may contain generics
            let last_segment = type_path.path.segments.last().unwrap();

            // Create a "clean" path without angle brackets for the implementation
            let service_type_clean = {
                let mut segments = type_path.path.segments.clone();

                // Only process the last segment, keeping all other segments unchanged
                if let Some(last) = segments.last_mut() {
                    // Keep only the identifier, removing any angle brackets
                    last.arguments = syn::PathArguments::None;
                }

                let path = syn::Path {
                    leading_colon: type_path.path.leading_colon,
                    segments,
                };

                quote! { #path }
            };

            // Extract the raw text of the type with its generics
            let full_type_str = quote!(#service_type).to_string();

            // Process generics only if we have angle bracketed arguments
            match &last_segment.arguments {
                syn::PathArguments::AngleBracketed(_) => {
                    // Extract the raw generics part from the angle brackets
                    if let Some(start) = full_type_str.find('<') {
                        if let Some(end) = full_type_str.rfind('>') {
                            let generics_text = full_type_str[start + 1..end].trim();

                            // Parse the generic parameters
                            let mut generic_entries = Vec::new();

                            // Handle the case with nested angle brackets
                            let mut current = String::new();
                            let mut angle_depth = 0;

                            for c in generics_text.chars() {
                                match c {
                                    '<' => {
                                        angle_depth += 1;
                                        current.push(c);
                                    }
                                    '>' => {
                                        angle_depth -= 1;
                                        current.push(c);
                                    }
                                    ',' if angle_depth == 0 => {
                                        if !current.trim().is_empty() {
                                            generic_entries.push(current.trim().to_string());
                                        }
                                        current.clear();
                                    }
                                    _ => current.push(c),
                                }
                            }

                            if !current.trim().is_empty() {
                                generic_entries.push(current.trim().to_string());
                            }

                            let mut generic_params = Vec::new();
                            let mut where_bounds = Vec::new();

                            // Process each generic entry
                            for entry in &generic_entries {
                                // If the entry has a colon, it has constraints
                                if let Some(colon_idx) = entry.find(':') {
                                    let param_name = &entry[..colon_idx].trim();
                                    let full_constraint = entry.clone();

                                    if let Ok(ident) = syn::parse_str::<syn::Ident>(param_name) {
                                        generic_params.push(ident);

                                        // Create a where predicate for the constraints
                                        if let Ok(predicate) =
                                            syn::parse_str::<syn::WherePredicate>(&full_constraint)
                                        {
                                            where_bounds.push(quote!(#predicate));
                                        }
                                    }
                                } else {
                                    // Simple generic parameter without constraints
                                    if let Ok(ident) = syn::parse_str::<syn::Ident>(entry) {
                                        generic_params.push(ident);
                                    }
                                }
                            }

                            // Generate the impl and type generics
                            let impl_generics = if !generic_params.is_empty() {
                                quote! { <#(#generic_params),*> }
                            } else {
                                quote! {}
                            };

                            // Generate the type generics (used in ActorHandle<Service<T>>)
                            let type_generics = if !generic_params.is_empty() {
                                quote! { <#(#generic_params),*> }
                            } else {
                                quote! {}
                            };

                            // Generate the where clause if we have any bounds
                            let where_clause = if !where_bounds.is_empty() {
                                quote! {
                                    where
                                        #(#where_bounds),*
                                }
                            } else {
                                quote! {}
                            };

                            return (
                                quote! { #service_type_clean #type_generics },
                                impl_generics,
                                type_generics,
                                where_clause,
                                generic_params,
                                where_bounds,
                            );
                        }
                    }

                    // Fallback to default if we couldn't parse the generics properly
                    default_result
                }

                // For path with no angle brackets (simple types)
                _ => (
                    quote! { #service_type },
                    quote! {},
                    quote! {},
                    quote! {},
                    Vec::new(),
                    Vec::new(),
                ),
            }
        }
        // Fallback for any other type - just use it as is
        _ => default_result,
    }
}

/// An attribute macro for specifying behaviour for individual command variants.
///
/// This macro is used in conjunction with the `actor_command` attribute macro to specify
/// the behaviour of individual command variants.
///
/// # Parameters
///
/// - `mode`: (Optional) Override the default command mode ("FireAndForget", "ImmediateResponse", or "AsyncResponse")
/// - `success_type`: (Optional) The success type returned in the Result
/// - `error_type`: (Optional) Override the default error type
/// - `inner_channel_type`: (Optional) Override the default channel type for AsyncResponse mode
///
/// # Usage
///
/// ```ignore
/// #[actor_command(
///     service = BlockchainService,
///     default_mode = "ImmediateResponse"
/// )]
/// pub enum BlockchainServiceCommand {
///     // Using default mode (ImmediateResponse) with specified success type
///     #[command(success_type = SubmittedTransaction)]
///     SendExtrinsic {
///         call: RuntimeCall,
///         options: SendExtrinsicOptions,
///     },
///     
///     // Overriding mode to AsyncResponse with custom inner channel type
///     #[command(
///         mode = "AsyncResponse",
///         success_type = BlockNumber,
///         error_type = ApiError,
///         inner_channel_type = tokio::sync::oneshot::Receiver
///     )]
///     WaitForBlock {
///         block_number: BlockNumber,
///     },
///     
///     // Using FireAndForget mode (no response expected)
///     #[command(mode = "FireAndForget")]
///     UnwatchExtrinsic {
///         subscription_id: Number,
///     }
/// }
/// ```
///
/// This is a marker attribute that is processed by the `actor_command` macro.
/// When used, it signals that a command variant has specific behaviour requirements.
#[proc_macro_attribute]
pub fn command(_args: TokenStream, _input: TokenStream) -> TokenStream {
    // This is just a marker attribute that is processed by the actor_command macro
    // Return the input unchanged
    _input
}
