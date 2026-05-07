//! Derive macro for [`gpui_preview::Previewable`].
//!
//! This crate provides `#[derive(Previewable)]` which generates prop editor
//! metadata and automatic component registration via [`inventory`] for use
//! with the `gpui-preview` app.
//!
//! You normally don't depend on this crate directly — it's re-exported from
//! `gpui-preview`.
//!
//! ## Struct attributes
//!
//! | Attribute | Description |
//! |-----------|-------------|
//! | `#[preview(category = "...")]` | Sidebar grouping in the preview app |
//! | `#[preview(no_register)]` | Skip automatic `inventory` registration |
//!
//! ## Field attributes
//!
//! | Attribute | Description |
//! |-----------|-------------|
//! | `#[preview(skip)]` | Exclude field from the prop editor |
//! | `#[preview(slider(min = 0.0, max = 100.0))]` | Render as a slider control |

use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Expr, Fields, Lit, Meta, parse_macro_input};

#[proc_macro_derive(Previewable, attributes(preview))]
pub fn derive_previewable(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match &input.data {
        Data::Struct(data) => derive_struct(&input, data),
        Data::Enum(data) => derive_enum(&input, data),
        Data::Union(_) => {
            syn::Error::new_spanned(&input, "Previewable cannot be derived for unions")
                .to_compile_error()
                .into()
        }
    }
}

// ── Struct derive ──────────────────────────────────────────────────────

fn derive_struct(input: &DeriveInput, data: &syn::DataStruct) -> TokenStream {
    let name = &input.ident;
    let name_str = name.to_string();

    let no_register = has_preview_flag(&input.attrs, "no_register");

    let category =
        extract_preview_kv(&input.attrs, "category").unwrap_or_else(|| "Uncategorized".to_string());
    let description = extract_doc_comment(&input.attrs);

    let Fields::Named(fields) = &data.fields else {
        return syn::Error::new_spanned(&input.ident, "Previewable requires named fields")
            .to_compile_error()
            .into();
    };

    let mut field_metas = Vec::new();
    let mut get_arms = Vec::new();
    let mut set_arms = Vec::new();

    for field in &fields.named {
        let field_ident = field.ident.as_ref().unwrap();
        let field_name = field_ident.to_string();

        if has_preview_flag(&field.attrs, "skip") {
            continue;
        }

        let doc = extract_doc_comment(&field.attrs);
        let ty = &field.ty;

        // Control kind
        let control = if let Some((min, max)) = extract_slider_attr(&field.attrs) {
            quote! { gpui_preview::ControlKind::NumberSlider { min: #min, max: #max } }
        } else {
            type_to_control(ty)
        };

        field_metas.push(quote! {
            gpui_preview::FieldMeta {
                name: #field_name,
                doc: #doc,
                control: #control,
            }
        });

        get_arms.push(type_to_get(field_ident, &field_name, ty));
        set_arms.push(type_to_set(field_ident, &field_name, ty));
    }

    let registration = if no_register {
        quote! {}
    } else {
        quote! {
            gpui_preview::inventory::submit! {
                gpui_preview::PreviewEntry {
                    id: || std::any::type_name::<#name>(),
                    name: #name_str,
                    category: #category,
                    description: #description,
                    fields: <#name as gpui_preview::Previewable>::fields,
                    create_default: || {
                        Box::new(<#name as gpui_preview::Previewable>::default_preview())
                    },
                    render: |any: &dyn std::any::Any| -> gpui::AnyElement {
                        let instance = any.downcast_ref::<#name>().expect("type mismatch in render");
                        gpui::IntoElement::into_any_element(gpui::Component::new(instance.clone()))
                    },
                }
            }
        }
    };

    let expanded = quote! {
        impl gpui_preview::Previewable for #name {
            fn name() -> &'static str { #name_str }
            fn category() -> &'static str { #category }
            fn description() -> &'static str { #description }

            fn default_preview() -> Self {
                Default::default()
            }

            fn fields() -> Vec<gpui_preview::FieldMeta> {
                vec![#(#field_metas),*]
            }

            fn get_field(&self, name: &str) -> Option<gpui_preview::FieldValue> {
                match name {
                    #(#get_arms)*
                    _ => None,
                }
            }

            fn set_field(&self, name: &str, value: gpui_preview::FieldValue) -> Self {
                let mut new = self.clone();
                match (name, value) {
                    #(#set_arms)*
                    _ => {}
                }
                new
            }
        }

        #registration
    };

    expanded.into()
}

// ── Enum derive ────────────────────────────────────────────────────────

fn derive_enum(input: &DeriveInput, data: &syn::DataEnum) -> TokenStream {
    let name = &input.ident;

    let mut variant_idents = Vec::new();
    let mut variant_strs = Vec::new();

    for variant in &data.variants {
        if !variant.fields.is_empty() {
            return syn::Error::new_spanned(
                variant,
                "Previewable enums must have only unit variants (no fields)",
            )
            .to_compile_error()
            .into();
        }
        variant_idents.push(&variant.ident);
        variant_strs.push(variant.ident.to_string());
    }

    let expanded = quote! {
        impl gpui_preview::PreviewEnum for #name {
            fn variants() -> &'static [&'static str] {
                &[#(#variant_strs),*]
            }

            fn to_variant_name(&self) -> &'static str {
                match self {
                    #(Self::#variant_idents => #variant_strs,)*
                }
            }

            fn from_variant_name(name: &str) -> Option<Self> {
                match name {
                    #(#variant_strs => Some(Self::#variant_idents),)*
                    _ => None,
                }
            }
        }
    };

    expanded.into()
}

// ── Type → ControlKind mapping ─────────────────────────────────────────

fn type_to_control(ty: &syn::Type) -> proc_macro2::TokenStream {
    if let Some(inner) = extract_option_inner(ty) {
        let inner_control = type_to_control(inner);
        return quote! { gpui_preview::ControlKind::Optional(Box::new(#inner_control)) };
    }

    let Some(type_name) = last_segment_name(ty) else {
        return quote! { gpui_preview::ControlKind::Unsupported };
    };

    match type_name.as_str() {
        "String" => quote! { gpui_preview::ControlKind::TextInput },
        "bool" => quote! { gpui_preview::ControlKind::Toggle },
        "f32" | "f64" => {
            quote! { gpui_preview::ControlKind::NumberSlider { min: 0.0, max: 100.0 } }
        }
        "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "isize" | "usize" => {
            quote! { gpui_preview::ControlKind::NumberSlider { min: 0.0, max: 100.0 } }
        }
        "Hsla" => quote! { gpui_preview::ControlKind::Color },
        _ => {
            // Assume it implements PreviewEnum
            quote! {
                gpui_preview::ControlKind::Select(
                    <#ty as gpui_preview::PreviewEnum>::variants().to_vec()
                )
            }
        }
    }
}

// ── Type → get_field arm ───────────────────────────────────────────────

fn type_to_get(
    field_ident: &syn::Ident,
    field_name: &str,
    ty: &syn::Type,
) -> proc_macro2::TokenStream {
    if let Some(inner) = extract_option_inner(ty) {
        let inner_get = type_to_get_expr(field_ident, inner, true);
        return quote! {
            #field_name => match &self.#field_ident {
                Some(_opt_inner) => #inner_get,
                None => Some(gpui_preview::FieldValue::None),
            },
        };
    }

    let Some(type_name) = last_segment_name(ty) else {
        return quote! { #field_name => None, };
    };

    match type_name.as_str() {
        "String" => quote! {
            #field_name => Some(gpui_preview::FieldValue::String(self.#field_ident.clone())),
        },
        "bool" => quote! {
            #field_name => Some(gpui_preview::FieldValue::Bool(self.#field_ident)),
        },
        "f32" | "f64" => quote! {
            #field_name => Some(gpui_preview::FieldValue::Float(self.#field_ident as f64)),
        },
        "i8" | "i16" | "i32" | "i64" | "isize" | "u8" | "u16" | "u32" | "u64" | "usize" => {
            quote! {
                #field_name => Some(gpui_preview::FieldValue::Int(self.#field_ident as i64)),
            }
        }
        "Hsla" => quote! {
            #field_name => {
                let rgba: gpui::Rgba = self.#field_ident.into();
                Some(gpui_preview::FieldValue::Color([
                    (rgba.r * 255.0) as u8,
                    (rgba.g * 255.0) as u8,
                    (rgba.b * 255.0) as u8,
                    (rgba.a * 255.0) as u8,
                ]))
            },
        },
        _ => quote! {
            #field_name => Some(gpui_preview::FieldValue::Enum(
                gpui_preview::PreviewEnum::to_variant_name(&self.#field_ident).to_string()
            )),
        },
    }
}

fn type_to_get_expr(
    field_ident: &syn::Ident,
    ty: &syn::Type,
    is_option_inner: bool,
) -> proc_macro2::TokenStream {
    let src = if is_option_inner {
        quote! { _opt_inner }
    } else {
        quote! { self.#field_ident }
    };

    let Some(type_name) = last_segment_name(ty) else {
        return quote! { None };
    };

    match type_name.as_str() {
        "String" => quote! { Some(gpui_preview::FieldValue::String(#src.clone())) },
        "bool" => quote! { Some(gpui_preview::FieldValue::Bool(*#src)) },
        "f32" | "f64" => quote! { Some(gpui_preview::FieldValue::Float(*#src as f64)) },
        "i8" | "i16" | "i32" | "i64" | "isize" | "u8" | "u16" | "u32" | "u64" | "usize" => {
            quote! { Some(gpui_preview::FieldValue::Int(*#src as i64)) }
        }
        "Hsla" => quote! {
            {
                let rgba: gpui::Rgba = (*#src).into();
                Some(gpui_preview::FieldValue::Color([
                    (rgba.r * 255.0) as u8,
                    (rgba.g * 255.0) as u8,
                    (rgba.b * 255.0) as u8,
                    (rgba.a * 255.0) as u8,
                ]))
            }
        },
        _ => quote! {
            Some(gpui_preview::FieldValue::Enum(
                gpui_preview::PreviewEnum::to_variant_name(#src).to_string()
            ))
        },
    }
}

// ── Type → set_field arm ───────────────────────────────────────────────

fn type_to_set(
    field_ident: &syn::Ident,
    field_name: &str,
    ty: &syn::Type,
) -> proc_macro2::TokenStream {
    if let Some(inner) = extract_option_inner(ty) {
        let inner_set = type_to_set_expr(field_ident, inner, true);
        return quote! {
            (#field_name, gpui_preview::FieldValue::None) => { new.#field_ident = None; }
            #inner_set
        };
    }

    let Some(type_name) = last_segment_name(ty) else {
        return quote! {};
    };

    match type_name.as_str() {
        "String" => quote! {
            (#field_name, gpui_preview::FieldValue::String(v)) => { new.#field_ident = v; }
        },
        "bool" => quote! {
            (#field_name, gpui_preview::FieldValue::Bool(v)) => { new.#field_ident = v; }
        },
        "f32" => quote! {
            (#field_name, gpui_preview::FieldValue::Float(v)) => { new.#field_ident = v as f32; }
        },
        "f64" => quote! {
            (#field_name, gpui_preview::FieldValue::Float(v)) => { new.#field_ident = v; }
        },
        "i8" | "i16" | "i32" | "i64" | "isize" | "u8" | "u16" | "u32" | "u64" | "usize" => {
            quote! {
                (#field_name, gpui_preview::FieldValue::Int(v)) => { new.#field_ident = v as #ty; }
            }
        }
        "Hsla" => quote! {
            (#field_name, gpui_preview::FieldValue::Color(v)) => {
                new.#field_ident = gpui::Hsla::from(gpui::Rgba {
                    r: v[0] as f32 / 255.0,
                    g: v[1] as f32 / 255.0,
                    b: v[2] as f32 / 255.0,
                    a: v[3] as f32 / 255.0,
                });
            }
        },
        _ => quote! {
            (#field_name, gpui_preview::FieldValue::Enum(ref v)) => {
                if let Some(e) = <#ty as gpui_preview::PreviewEnum>::from_variant_name(v) {
                    new.#field_ident = e;
                }
            }
        },
    }
}

fn type_to_set_expr(
    field_ident: &syn::Ident,
    inner_ty: &syn::Type,
    _is_option: bool,
) -> proc_macro2::TokenStream {
    let Some(type_name) = last_segment_name(inner_ty) else {
        return quote! {};
    };

    let field_name = field_ident.to_string();

    match type_name.as_str() {
        "String" => quote! {
            (#field_name, gpui_preview::FieldValue::String(v)) => { new.#field_ident = Some(v); }
        },
        "bool" => quote! {
            (#field_name, gpui_preview::FieldValue::Bool(v)) => { new.#field_ident = Some(v); }
        },
        "f32" => quote! {
            (#field_name, gpui_preview::FieldValue::Float(v)) => { new.#field_ident = Some(v as f32); }
        },
        "f64" => quote! {
            (#field_name, gpui_preview::FieldValue::Float(v)) => { new.#field_ident = Some(v); }
        },
        "i8" | "i16" | "i32" | "i64" | "isize" | "u8" | "u16" | "u32" | "u64" | "usize" => {
            quote! {
                (#field_name, gpui_preview::FieldValue::Int(v)) => { new.#field_ident = Some(v as #inner_ty); }
            }
        }
        "Hsla" => quote! {
            (#field_name, gpui_preview::FieldValue::Color(v)) => {
                new.#field_ident = Some(gpui::Hsla::from(gpui::Rgba {
                    r: v[0] as f32 / 255.0,
                    g: v[1] as f32 / 255.0,
                    b: v[2] as f32 / 255.0,
                    a: v[3] as f32 / 255.0,
                }));
            }
        },
        _ => quote! {
            (#field_name, gpui_preview::FieldValue::Enum(ref v)) => {
                if let Some(e) = <#inner_ty as gpui_preview::PreviewEnum>::from_variant_name(v) {
                    new.#field_ident = Some(e);
                }
            }
        },
    }
}

// ── Attribute helpers ──────────────────────────────────────────────────

fn extract_preview_kv(attrs: &[syn::Attribute], key: &str) -> Option<String> {
    for attr in attrs {
        if !attr.path().is_ident("preview") {
            continue;
        }
        let Ok(nested) = attr
            .parse_args_with(syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated)
        else {
            continue;
        };
        for meta in &nested {
            if let Meta::NameValue(nv) = meta
                && nv.path.is_ident(key)
                && let Expr::Lit(expr_lit) = &nv.value
                && let Lit::Str(lit_str) = &expr_lit.lit
            {
                return Some(lit_str.value());
            }
        }
    }
    None
}

fn has_preview_flag(attrs: &[syn::Attribute], flag: &str) -> bool {
    for attr in attrs {
        if !attr.path().is_ident("preview") {
            continue;
        }
        let Ok(nested) = attr
            .parse_args_with(syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated)
        else {
            continue;
        };
        for meta in &nested {
            if let Meta::Path(path) = meta
                && path.is_ident(flag)
            {
                return true;
            }
        }
    }
    false
}

fn extract_slider_attr(attrs: &[syn::Attribute]) -> Option<(f64, f64)> {
    for attr in attrs {
        if !attr.path().is_ident("preview") {
            continue;
        }
        let Ok(nested) = attr
            .parse_args_with(syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated)
        else {
            continue;
        };
        for meta in &nested {
            if let Meta::List(list) = meta
                && list.path.is_ident("slider")
            {
                let Ok(inner) = list.parse_args_with(
                    syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated,
                ) else {
                    continue;
                };
                let mut min = 0.0f64;
                let mut max = 100.0f64;
                for m in &inner {
                    if let Meta::NameValue(nv) = m
                        && let Expr::Lit(expr_lit) = &nv.value
                        && let Lit::Float(f) = &expr_lit.lit
                    {
                        let val: f64 = f.base10_parse().unwrap();
                        if nv.path.is_ident("min") {
                            min = val;
                        } else if nv.path.is_ident("max") {
                            max = val;
                        }
                    }
                }
                return Some((min, max));
            }
        }
    }
    None
}

fn extract_doc_comment(attrs: &[syn::Attribute]) -> String {
    let mut lines = Vec::new();
    for attr in attrs {
        if !attr.path().is_ident("doc") {
            continue;
        }
        if let Meta::NameValue(nv) = &attr.meta
            && let Expr::Lit(expr_lit) = &nv.value
            && let Lit::Str(lit_str) = &expr_lit.lit
        {
            lines.push(lit_str.value().trim().to_string());
        }
    }
    lines.join(" ")
}

fn last_segment_name(ty: &syn::Type) -> Option<String> {
    if let syn::Type::Path(type_path) = ty {
        type_path.path.segments.last().map(|s| s.ident.to_string())
    } else {
        None
    }
}

fn extract_option_inner(ty: &syn::Type) -> Option<&syn::Type> {
    let syn::Type::Path(type_path) = ty else {
        return None;
    };
    let segment = type_path.path.segments.last()?;
    if segment.ident != "Option" {
        return None;
    }
    let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
        return None;
    };
    let syn::GenericArgument::Type(inner) = args.args.first()? else {
        return None;
    };
    Some(inner)
}
