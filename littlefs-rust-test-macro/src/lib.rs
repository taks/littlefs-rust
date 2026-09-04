#![allow(clippy::single_match)]

use syn::{FnArg, ItemFn, punctuated::Punctuated, token::Comma};

#[proc_macro_attribute]
pub fn lfs_test(
    attr: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let mut input_fn = syn::parse_macro_input!(input as ItemFn);

    let f_name = format!("{}_", input_fn.sig.ident);
    let f_ident = syn::Ident::new(&f_name, proc_macro2::Span::call_site());
    // dbg!(ident);
    let call_fn = input_fn.sig.ident.clone();
    let attrs = std::mem::take(&mut input_fn.attrs);

    let args: Punctuated<FnArg, Comma> = input_fn.sig.inputs.clone().into_iter().skip(1).collect();
    let args_ = args
        .clone()
        .into_iter()
        .map(|arg| match arg {
            FnArg::Typed(pat_type) => {
                let pat = *pat_type.pat;
                match pat {
                    syn::Pat::Ident(pat_ident) => pat_ident.ident,
                    _ => panic!("Expected typed argument"),
                }
            }
            _ => panic!("Expected typed argument"),
        })
        .collect::<Punctuated<_, Comma>>();

    for input in &mut input_fn.sig.inputs {
        match input {
            FnArg::Typed(pat_type) => {
                pat_type.attrs.clear();
            }
            _ => {}
        };
    }

    let reentrant = if attr.is_empty() {
        false
    } else {
        let attr = syn::parse_macro_input!(attr as syn::Ident);
        attr == "reentrant"
    };
    let reentrant = if reentrant {
        quote::quote! {
            run_powerloss_linear(&mut cfg, |cfg| {
                #call_fn(cfg, #args_);
            });
        }
    } else {
        quote::quote! {}
    };

    quote::quote! {
        #[rstest::rstest]
        #(#attrs)*
        fn #f_ident(#args) {
            use std::ptr::NonNull;
            use common::{init_logger, run_powerloss_none, run_powerloss_linear};

            init_logger();

            for (size_, block_size) in [
                (16, 512),
                (1, 512),
                (512, 512),
                (1, 4096),
                (4096, 32768)] {

                let read_buf = vec![0u8; block_size as usize];
                let prog_buf = vec![0u8; block_size as usize];
                let lookahead_buf = vec![0u8; block_size as usize];
                let mut cfg = LfsConfig {
                    context: None,
                    read_size: size_,
                    prog_size: size_,
                    block_size,
                    block_count: 128,
                    block_cycles: -1,
                    cache_size: block_size,
                    compact_thresh: u32::MAX,
                    read_buffer: Some(NonNull::from_ref(&read_buf)),
                    prog_buffer: Some(NonNull::from_ref(&prog_buf)),
                    lookahead_buffer: Some(NonNull::from_ref(&lookahead_buf)),
                    name_max: 255,
                    file_max: 2_147_483_647,
                    attr_max: 1022,
                    metadata_max: 0,
                    inline_max: 0,
                };

                run_powerloss_none(&mut cfg, |cfg| {
                    #call_fn(cfg, #args_);
                });

                #reentrant
            }
        }

        #[cfg(test)]
        #input_fn
    }
    .into()
}
