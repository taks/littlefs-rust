use syn::{FnArg, ItemFn, punctuated::Punctuated, token::Comma};

#[proc_macro_attribute]
pub fn littlefs_test(
    _attr: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let mut input_fn = syn::parse_macro_input!(input as ItemFn);

    let f_name = format!("{}_", input_fn.sig.ident);
    let f_ident = syn::Ident::new(&f_name, proc_macro2::Span::call_site());
    // dbg!(ident);
    let call_fn = input_fn.sig.ident.clone();

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

    quote::quote! {
        #[rstest::rstest]
        fn #f_ident(#args) {
            init_logger();

            for (size, block_size) in [
                (16, 512),
                (1, 512),
                (512, 512),
                (1, 4096),
                (4096, 32768)] {

                let mut env = config_with_geometry(block_size, 128);
                env.config.read_size = size;
                env.config.prog_size = size;
                init_context(&mut env);

                #call_fn(&mut env.config, #args_);
            }
        }

        #[cfg(test)]
        #input_fn
    }
    .into()
}
