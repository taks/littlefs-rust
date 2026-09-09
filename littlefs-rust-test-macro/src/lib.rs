#![allow(clippy::single_match)]

use syn::{
    FnArg, ItemFn, Pat,
    parse::Parser,
    punctuated::Punctuated,
    token::{self, Comma},
};

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
    let args_idents: Vec<_> = args
        .clone()
        .into_iter()
        .map(|arg| match arg {
            FnArg::Typed(pat_type) => {
                let pat = *pat_type.pat;
                match pat {
                    Pat::Ident(pat_ident) => pat_ident.ident,
                    _ => panic!("Expected typed argument"),
                }
            }
            _ => panic!("Expected typed argument"),
        })
        .collect();

    let assign = syn::parse_macro_input!(attr with Punctuated::<syn::ExprAssign, token::Semi>::parse_terminated);
    let assign2 = quote::quote! {
        let reentrant = false;
        let block_cycles = -1;
        let inline_max = 0;
        let compact_thresh = u32::MAX;
        let name_max = 255;
        let cache_size = 64.max(read_size);
        let lookahead_size = 16;

        let erase_count = 1024 * 1024 / erase_size;
        let block_size = erase_size;
        let erase_value = Some(0xFF);
        let erase_cycles = 0;
        let badblock_behavior = BadblockBehavior::ProgError;
        let block_count = erase_count / std::cmp::max(block_size/erase_size, 1);

    };
    let mut cfg_params = Vec::new();
    let assign2: Punctuated<syn::ExprLet, token::Semi> =
        Punctuated::<syn::ExprLet, token::Semi>::parse_terminated
            .parse2(assign2)
            .unwrap()
            .into_iter()
            .filter(|p| match &*p.pat {
                syn::Pat::Ident(i) => {
                    if args_idents.iter().any(|a| *a == i.ident) {
                        cfg_params.push(i.ident.clone());
                        false
                    } else {
                        true
                    }
                }
                _ => false,
            })
            .collect();

    input_fn.sig.inputs = input_fn
        .sig
        .inputs
        .into_iter()
        .filter_map(|input| match input {
            FnArg::Typed(mut pat_type) => {
                if let Pat::Ident(ref pat_ident) = *pat_type.pat
                    && cfg_params.iter().any(|i| *i == pat_ident.ident)
                {
                    None
                } else {
                    pat_type.attrs.clear();
                    Some(FnArg::Typed(pat_type))
                }
            }
            _ => Some(input),
        })
        .collect();

    let args_: Punctuated<_, Comma> = args_idents
        .iter()
        .filter(|i| !cfg_params.iter().any(|p| *p == **i))
        .collect();

    quote::quote! {
        #[rstest::rstest]
        #(#attrs)*
        fn #f_ident(#args) {
            use std::ptr::NonNull;
            use common::{init_logger, run_powerloss_none, run_powerloss_linear,
                EmubdConfig, BadblockBehavior, PowerLossBehavior};

            init_logger();

            for (read_size, erase_size) in [
                (16, 512),
                (1, 512),
                (512, 512),
                (1, 4096),
                (4096, 32768)
            ] {

                #assign2;
                #assign;

                let read_buf = vec![0u8; cache_size as usize];
                let prog_buf = vec![0u8; cache_size as usize];
                let lookahead_buf = vec![0u8; lookahead_size as usize];

                let mut cfg = LfsConfig {
                    context: None,
                    read_size,
                    prog_size: read_size,
                    block_size,
                    block_count,
                    block_cycles,
                    cache_size,
                    compact_thresh,
                    read_buffer: Some(NonNull::from_ref(&read_buf)),
                    prog_buffer: Some(NonNull::from_ref(&prog_buf)),
                    lookahead_buffer: Some(NonNull::from_ref(&lookahead_buf)),
                    name_max,
                    file_max: 2_147_483_647,
                    attr_max: 1022,
                    metadata_max: 0,
                    inline_max,
                };

                let bdcfg = EmubdConfig {
                    read_size,
                    prog_size: read_size,
                    erase_size: block_size,
                    erase_count,
                    erase_value,
                    erase_cycles,
                    badblock_behavior,
                    power_cycles: 0,
                    powerloss_behavior: PowerLossBehavior::Noop,
                    powerloss_cb: &|| {},
                };

                if reentrant {
                    run_powerloss_linear(&mut cfg, &bdcfg, |cfg| {
                        #call_fn(cfg, #args_);
                    });
                } else {
                    run_powerloss_none(&mut cfg, &bdcfg, |cfg| {
                        #call_fn(cfg, #args_);
                    });
                }
            }
        }

        #[cfg(test)]
        #input_fn
    }
    .into()
}
