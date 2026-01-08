# MetricLabels Implementation (Removed - For Future Reference)

This document describes the `#[metric_label]` infrastructure that was removed from the codebase. It can be used to re-implement this feature when structured logging with trace correlation is needed.

## Purpose

The `#[metric_label]` system was designed to:
- Extract specific field values from commands and events for structured logging
- Enable correlation between Prometheus metrics (aggregates) and logs (details)
- Support trace_id-based debugging in OpenSearch/logging systems

**Design Decision:** Metrics are for aggregates (counts, latencies by operation type), logs are for details (specific file_key, provider_id values with trace_id).

## Components

### 1. MetricLabels Trait

**Location:** `client/actors-framework/src/metrics.rs`

```rust
/// Unified trait for both commands and events to specify metric labels.
/// Auto-implemented by `#[actor_command]` and `ActorEvent` derive macros.
pub trait MetricLabels {
    /// Name in snake_case (auto-derived from type/variant name)
    fn metric_name(&self) -> &'static str;

    /// Custom labels for this instance (e.g., file_key, provider_id).
    /// Only fields marked with `#[metric_label]` attribute are included.
    fn metric_labels(&self) -> Vec<(&'static str, String)>;
}
```

### 2. MetricLabelFormat Enum

**Location:** `client/actors-derive/src/lib.rs`

```rust
enum MetricLabelFormat {
    /// Use Debug format `{:?}` (default)
    Debug,
    /// Use hex format `0x{:x}` (for LowerHex types)
    Hex,
    /// Use Display format `{}` (for Display types)
    Display,
}
```

### 3. Format Extraction Function

```rust
/// Extract metric_label attribute from a field and parse its format.
fn extract_metric_label_format(attrs: &[Attribute]) -> Option<MetricLabelFormat> {
    for attr in attrs {
        if attr.path().is_ident("metric_label") {
            // Check if it's just #[metric_label] or #[metric_label(format)]
            match &attr.meta {
                syn::Meta::Path(_) => {
                    // Just #[metric_label] - use default Debug format
                    return Some(MetricLabelFormat::Debug);
                }
                syn::Meta::List(meta_list) => {
                    // #[metric_label(format)] - parse the format
                    let tokens_str = meta_list.tokens.to_string();
                    let format_str = tokens_str.trim();
                    match format_str {
                        "hex" => return Some(MetricLabelFormat::Hex),
                        "display" => return Some(MetricLabelFormat::Display),
                        "debug" | "" => return Some(MetricLabelFormat::Debug),
                        _ => {
                            // Unknown format, default to Debug
                            return Some(MetricLabelFormat::Debug);
                        }
                    }
                }
                _ => {
                    // Any other form, default to Debug
                    return Some(MetricLabelFormat::Debug);
                }
            }
        }
    }
    None
}
```

### 4. Format Code Generation

```rust
/// Generate format code for a metric label based on its format specifier.
fn generate_label_format_code(
    field_name: &Ident,
    format: MetricLabelFormat,
    use_self: bool,
) -> proc_macro2::TokenStream {
    let field_access = if use_self {
        quote! { self.#field_name }
    } else {
        quote! { #field_name }
    };

    match format {
        MetricLabelFormat::Debug => {
            quote! { format!("{:?}", #field_access) }
        }
        MetricLabelFormat::Hex => {
            quote! { format!("0x{:x}", #field_access) }
        }
        MetricLabelFormat::Display => {
            quote! { format!("{}", #field_access) }
        }
    }
}
```

### 5. Struct Field Extraction (for ActorEvent)

```rust
/// Extract metric labels from struct fields with #[metric_label] attribute.
fn extract_metric_labels_from_struct(data: &syn::Data) -> Vec<proc_macro2::TokenStream> {
    let mut labels = Vec::new();

    if let syn::Data::Struct(data_struct) = data {
        if let syn::Fields::Named(fields_named) = &data_struct.fields {
            for field in &fields_named.named {
                if let Some(field_name) = &field.ident {
                    if let Some(format) = extract_metric_label_format(&field.attrs) {
                        let field_name_str = field_name.to_string();
                        let format_code = generate_label_format_code(field_name, format, true);
                        labels.push(quote! {
                            (#field_name_str, #format_code)
                        });
                    }
                }
            }
        }
    }

    labels
}
```

### 6. Variant Field Extraction (for actor_command)

```rust
/// Extract metric labels from variant fields with #[metric_label] attribute.
/// Returns a list of (field_name, format) pairs for fields that have the attribute.
fn extract_metric_labels_from_variant_fields(fields: &syn::Fields) -> Vec<(Ident, MetricLabelFormat)> {
    let mut labels = Vec::new();

    if let syn::Fields::Named(fields_named) = fields {
        for field in &fields_named.named {
            if let Some(field_name) = &field.ident {
                if let Some(format) = extract_metric_label_format(&field.attrs) {
                    labels.push((field_name.clone(), format));
                }
            }
        }
    }

    labels
}

/// Generate metric_labels code for a variant's fields.
/// Returns (field_patterns, label_code) for use in match arm.
fn generate_metric_labels_for_variant(
    fields: &syn::Fields,
) -> (Vec<proc_macro2::TokenStream>, Vec<proc_macro2::TokenStream>) {
    let mut field_patterns = Vec::new();
    let mut label_code = Vec::new();

    if let syn::Fields::Named(fields_named) = fields {
        for field in &fields_named.named {
            if let Some(field_name) = &field.ident {
                if let Some(format) = extract_metric_label_format(&field.attrs) {
                    // Add field to pattern
                    field_patterns.push(quote! { #field_name });

                    // Generate the label entry
                    let field_name_str = field_name.to_string();
                    let format_code = generate_label_format_code(field_name, format, false);
                    label_code.push(quote! {
                        (#field_name_str, #format_code)
                    });
                }
            }
        }
    }

    (field_patterns, label_code)
}
```

### 7. Proc Macro Attribute

```rust
/// An attribute macro for marking fields that should be included as metric labels.
///
/// # Format Options
///
/// - `#[metric_label]` - Default Debug format `{:?}`
/// - `#[metric_label(hex)]` - Hex format `0x{:x}` (for types implementing LowerHex)
/// - `#[metric_label(display)]` - Display format `{}` (for types implementing Display)
#[proc_macro_attribute]
pub fn metric_label(_args: TokenStream, input: TokenStream) -> TokenStream {
    // This is just a marker attribute that is processed by the ActorEvent and actor_command macros
    // Return the input unchanged
    input
}
```

## Usage Examples

### Events

```rust
#[derive(Debug, Clone, ActorEvent)]
#[actor(actor = "blockchain_service")]
pub struct NewStorageRequest {
    #[metric_label(hex)]  // Will format as "0x{:x}"
    pub file_key: H256,
    #[metric_label]       // Will format as "{:?}"
    pub provider_id: ProviderId,
    pub location: FileLocation,  // Not a label (no attribute)
}
```

### Commands

```rust
#[actor_command(service = MyService, default_mode = "ImmediateResponse")]
pub enum MyCommand {
    DoSomething {
        #[metric_label(display)]  // Will format as "{}"
        count: u64,
        #[metric_label(hex)]
        file_key: H256,
    },
}
```

## Generated Code

For the `NewStorageRequest` example above, the macro generates:

```rust
impl MetricLabels for NewStorageRequest {
    fn metric_name(&self) -> &'static str {
        "new_storage_request"
    }

    fn metric_labels(&self) -> Vec<(&'static str, String)> {
        vec![
            ("file_key", format!("0x{:x}", self.file_key)),
            ("provider_id", format!("{:?}", self.provider_id)),
        ]
    }
}
```

## Integration Points (Not Implemented)

To actually use this infrastructure, you would add logging at command/event entry points:

```rust
// Example: In event handler before processing
fn handle_event(&mut self, event: E) -> impl Future<Output = Result<String>> {
    let trace_id = generate_trace_id();
    let labels = event.metric_labels();

    info!(
        target: "telemetry",
        trace_id = %trace_id,
        event = %event.metric_name(),
        labels = ?labels,
        "Event received"
    );

    // ... process event ...

    info!(
        target: "telemetry",
        trace_id = %trace_id,
        event = %event.metric_name(),
        duration_ms = %duration.as_millis(),
        "Event completed"
    );
}
```

This enables correlation between:
- **Prometheus metrics**: `storagehub_event_handler_seconds{event="new_storage_request"}`
- **Structured logs**: `{"trace_id": "abc123", "event": "new_storage_request", "file_key": "0x1234...", "provider_id": "..."}`

## Re-implementation Checklist

1. Add `MetricLabels` trait to `client/actors-framework/src/metrics.rs`
2. Add `MetricLabelFormat` enum to `client/actors-derive/src/lib.rs`
3. Add helper functions: `extract_metric_label_format`, `generate_label_format_code`, `extract_metric_labels_from_struct`, `extract_metric_labels_from_variant_fields`, `generate_metric_labels_for_variant`
4. Add `#[proc_macro_attribute] pub fn metric_label`
5. Update `ActorEvent` derive macro to generate `MetricLabels` impl
6. Update `actor_command` macro to generate `MetricLabels` impl
7. Add `metric_label` to the `attributes` list in both macros
8. Implement actual logging integration that calls `metric_labels()`
