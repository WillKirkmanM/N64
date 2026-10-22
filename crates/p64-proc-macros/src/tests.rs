use super::*;

#[test]
fn instrumentation_preserves_attributes_signature_and_early_return_body() {
    let original: ItemFn = parse_quote! {
        #[inline(always)]
        #[doc = "Cache access"]
        pub fn cache_access(n64: &mut Device, value: u32) -> u32 {
            if value == 0 { return 7; }
            value
        }
    };
    let expanded: ItemFn = syn::parse2(with_cycle_charge(original.clone(), quote!(4))).unwrap();

    assert_eq!(expanded.attrs, original.attrs);
    assert_eq!(expanded.vis, original.vis);
    assert_eq!(expanded.sig, original.sig);
    assert_eq!(&expanded.block.stmts[1..], original.block.stmts);
    let charge: syn::Stmt = parse_quote!(crate::mmu::add_cycles(n64, 4););

    assert_eq!(expanded.block.stmts[0], charge);
}
