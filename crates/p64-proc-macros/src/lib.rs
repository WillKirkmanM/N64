use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, parse_quote, ItemFn};

fn with_cycle_charge(mut function: ItemFn, cycles: TokenStream2) -> TokenStream2 {
    function
        .block
        .stmts
        .insert(0, parse_quote!(crate::mmu::add_cycles(n64, #cycles);));
    quote!(#function)
}

#[proc_macro_attribute]
pub fn add_cycles(attr: TokenStream, item: TokenStream) -> TokenStream {
    let cycles = parse_macro_input!(attr as syn::LitInt);
    let function = parse_macro_input!(item as ItemFn);

    with_cycle_charge(function, quote!(#cycles)).into()
}

#[proc_macro_attribute]
pub fn add_rdram_cycles(attr: TokenStream, item: TokenStream) -> TokenStream {
    let length = parse_macro_input!(attr as syn::LitInt);
    let function = parse_macro_input!(item as ItemFn);

    with_cycle_charge(
        function,
        quote!(crate::rdram::rdram_calculate_cycles(#length)),
    )
    .into()
}

#[cfg(test)]
mod tests;
