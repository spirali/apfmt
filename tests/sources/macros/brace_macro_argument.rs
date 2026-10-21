// A macro call with braces as the only argument hugs it, like a block
fn f() -> Result<TokenStream> {
    Ok(quote! {
        #[doc(hidden)]
        pub struct #encoder_name<#lt> {
            #(#serialize_fields,)*
        }
    })
}

fn chain() {
    let actual = ModuleRenderer::new("character")
        .config(toml::toml! {
            [character]
            vicmd_symbol = "[V](bold green)"
        })
        .collect();
}

fn nested() {
    let e = Example {
        result: Some(Value::test_list(vec![Value::test_record(record! {
            "lang" => Value::test_string("Nu_test"),
            "gems" => Value::test_int(100),
        })])),
    };
    let setting = Settings {
        embedders: Setting::Set(maplit::btreemap! {
            S("manual") => SettingEmbeddingSettings { inner: Setting::Set(EmbeddingSettings {
                dimensions: Setting::Set(3),
            }) }
        }),
    };
}

// On one line when it fits
fn short() {
    f(m! { a b });
}

// The last of more arguments is not hugged
fn more() {
    f(aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa, m! {
        b
    });
}
