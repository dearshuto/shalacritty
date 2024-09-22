use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, ItemFn, Stmt};

#[proc_macro_derive(ProfileObject)]
pub fn profile_derive(input: TokenStream) -> TokenStream {
    let input = &parse_macro_input!(input as DeriveInput);

    // 構造体じゃなかったらエラー
    let syn::Data::Struct(v) = &input.data else {
        return syn::Error::new_spanned(&input.ident, "Must be struct type")
            .to_compile_error()
            .into();
    };

    Default::default()
}

/// shalacritty のパフォーマンス計測用のアトリビュート
/// 関数に付与することで関数の開始と終了タイミングで所定の処理を呼び出します
///
/// 対象の構造体が shalacritty_core::IProfilerSubject を満たす必要があります
#[proc_macro_attribute]
pub fn profile(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut ast = parse_macro_input!(item as ItemFn);

    let ident = &ast.sig.ident;

    let enter = quote! {
        {
            self.begin(&format!("{}::AA", stringify!(#ident)));
        }
    };
    let enter: TokenStream = enter.into();
    let enter = parse_macro_input!(enter as Stmt);

    let mut body = quote! {};
    for s in &ast.block.stmts {
        body = quote! {
            #body
            #s
        };
    }
    let body = quote! {
        let body = || { #body };
    };
    let body: TokenStream = body.into();
    let body = parse_macro_input!(body as Stmt);

    let exit = quote! {
        {
            let ret = body();
            self.end();
            ret
        }
    };
    let exit: TokenStream = exit.into();
    let exit = parse_macro_input!(exit as Stmt);

    ast.block.stmts.clear();
    ast.block.stmts.push(enter);
    ast.block.stmts.push(body);
    ast.block.stmts.push(exit);

    let gen = quote! {
        #ast
    };

    gen.into()
}
