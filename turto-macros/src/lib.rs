//! Proc macros for turto.
//!
//! The [`turto_command`] attribute is the single source of truth for a command's
//! help metadata. It sits *above* `#[poise::command]`, reads the real function
//! signature, and registers the command — so the help text and the actual command
//! parameters can never drift apart.

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Attribute, Expr, ExprLit, FnArg, ItemFn, Lit, LitStr, Meta, MetaNameValue, Pat, Token,
    parse_macro_input, punctuated::Punctuated, spanned::Spanned,
};

/// Discord's hard limit on a slash command's description, and so on `short`.
const SHORT_DESCRIPTION_LIMIT: usize = 100;

/// The arguments `#[turto_command]` accepts.
const ARGUMENTS: &[&str] = &["short", "long", "hide_in_help"];

/// Register a `#[poise::command]` with turto's command registry.
///
/// Place it directly above `#[poise::command(...)]`. `short` (the ≤100 char slash
/// preview), `long` (the `/help` embed body) and a `#[description = "..."]` on every
/// parameter are all required — omitting any of them is a compile error. Add the bare
/// flag `hide_in_help` to keep the command out of the `/help` command list.
///
/// The function is re-emitted untouched and the registration is submitted beside it,
/// reached only through `CommandKind` — there is no generated name to know.
///
/// Parameter names are resolved exactly like poise does (`#[rename = "..."]` if
/// present, otherwise the binding identifier), so the registration always matches the
/// real `poise::Command` parameters.
///
/// # Example
/// ```ignore
/// #[turto_command(short = "Start playback.", long = "Start playback. ...")]
/// #[poise::command(slash_command, guild_only)]
/// pub async fn play(
///     ctx: Context<'_>,
///     #[description = "Optional, the link to play"]
///     #[rename = "url"]
///     query: Option<String>,
/// ) -> Result<(), CommandError> { Ok(()) }
/// ```
#[proc_macro_attribute]
pub fn turto_command(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr with Punctuated::<Meta, Token![,]>::parse_terminated);
    let func = parse_macro_input!(item as ItemFn);

    match expand(args, func) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn expand(
    args: Punctuated<Meta, Token![,]>,
    func: ItemFn,
) -> syn::Result<proc_macro2::TokenStream> {
    // An argument this macro does not read would otherwise be silently ignored.
    for arg in &args {
        if !ARGUMENTS.iter().any(|name| arg.path().is_ident(name)) {
            return Err(syn::Error::new(
                arg.path().span(),
                format!("unknown #[turto_command] argument, expected one of {ARGUMENTS:?}"),
            ));
        }
    }

    let short = take_required_lit(&args, "short")?;
    let long = take_required_lit(&args, "long")?;
    let hide_in_help = has_flag(&args, "hide_in_help")?;

    // Discord rejects an over-long description when the command is registered, which
    // would only surface as a failed startup. Catch it here instead.
    let short_len = short.value().chars().count();
    if short_len > SHORT_DESCRIPTION_LIMIT {
        return Err(syn::Error::new(
            short.span(),
            format!(
                "`short` is {short_len} characters, but Discord allows at most \
                 {SHORT_DESCRIPTION_LIMIT} for a slash command description"
            ),
        ));
    }

    // The command name is the function name, matching poise's own default.
    let build = &func.sig.ident;
    let ident = build.to_string();
    let name = LitStr::new(ident.trim_start_matches("r#"), build.span());

    // Skip the first parameter (`ctx`); collect (name, description) for the rest.
    let mut names: Vec<LitStr> = Vec::new();
    let mut descriptions: Vec<LitStr> = Vec::new();

    for arg in func.sig.inputs.iter().skip(1) {
        let FnArg::Typed(pat_type) = arg else {
            return Err(syn::Error::new(arg.span(), "unexpected `self` parameter"));
        };

        names.push(param_name(pat_type)?);
        descriptions.push(attr_lit(&pat_type.attrs, "description").ok_or_else(|| {
            syn::Error::new(
                pat_type.span(),
                "every parameter must have a `#[description = \"...\"]` for #[turto_command]",
            )
        })?);
    }

    Ok(quote! {
        #func

        ::inventory::submit! {
            crate::models::command::CommandEntry {
                name: #name,
                short_description: #short,
                description: #long,
                hide_in_help: #hide_in_help,
                parameters: &[
                    #(
                        crate::models::command::ParamMeta {
                            name: #names,
                            description: #descriptions,
                        },
                    )*
                ],
                build: #build,
            }
        }
    })
}

/// Resolve a parameter's name the same way poise does: `#[rename = "..."]` wins,
/// otherwise the binding identifier (with a leading `r#` stripped).
fn param_name(pat_type: &syn::PatType) -> syn::Result<LitStr> {
    if let Some(renamed) = attr_lit(&pat_type.attrs, "rename") {
        return Ok(renamed);
    }
    if let Pat::Ident(ident) = &*pat_type.pat {
        let name = ident.ident.to_string();
        let name = name.strip_prefix("r#").unwrap_or(&name);
        return Ok(LitStr::new(name, ident.ident.span()));
    }
    Err(syn::Error::new(
        pat_type.pat.span(),
        "pattern parameters require an explicit `#[rename = \"...\"]`",
    ))
}

/// Pull a required string-literal argument (e.g. `short = "..."`) out of the attribute args.
fn take_required_lit(args: &Punctuated<Meta, Token![,]>, key: &str) -> syn::Result<LitStr> {
    for arg in args {
        if arg.path().is_ident(key) {
            let Meta::NameValue(MetaNameValue {
                value:
                    Expr::Lit(ExprLit {
                        lit: Lit::Str(s), ..
                    }),
                ..
            }) = arg
            else {
                return Err(syn::Error::new(
                    arg.span(),
                    format!("`{key}` must be a string literal, `{key} = \"...\"`"),
                ));
            };
            return Ok(s.clone());
        }
    }
    Err(syn::Error::new(
        proc_macro2::Span::call_site(),
        format!("#[turto_command] requires a string `{key} = \"...\"` argument"),
    ))
}

/// Whether a bare flag argument (e.g. `hide_in_help`) is present.
fn has_flag(args: &Punctuated<Meta, Token![,]>, key: &str) -> syn::Result<bool> {
    for arg in args {
        if arg.path().is_ident(key) {
            if !matches!(arg, Meta::Path(_)) {
                return Err(syn::Error::new(
                    arg.span(),
                    format!("`{key}` is a flag, write it as a bare `{key}` with no value"),
                ));
            }
            return Ok(true);
        }
    }
    Ok(false)
}

/// Find a `#[name = "literal"]` attribute and return its string literal, if present.
fn attr_lit(attrs: &[Attribute], name: &str) -> Option<LitStr> {
    for attr in attrs {
        if attr.path().is_ident(name)
            && let Meta::NameValue(nv) = &attr.meta
            && let Expr::Lit(ExprLit {
                lit: Lit::Str(s), ..
            }) = &nv.value
        {
            return Some(s.clone());
        }
    }
    None
}
