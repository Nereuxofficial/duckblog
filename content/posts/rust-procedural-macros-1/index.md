+++
title = "Making Code writing Code"
date = "2022-10-09T21:31:55+02:00"
tags = ["rust", "programming", "macros"]
keywords = ["rust", "programming", "macros"]
description = "An Introduction to procedural macros in Rust"
draft = true
showFullContent = false
+++

Macros are a really powerful tool for programming in Rust. You can think of them processing Rust code into 
other Rust code. The thing that sets procedural macros apart from declarative macros is that while the latter 
work similar to a match statement, procedural macros are functions that take a `TokenStream` as input and return
another TokenStream as output.

Here is a practical example of a declarative macro i used in a recent project to 
convert a `SendError` to our own error type `RLError`:

```rust
#[macro_export]
macro_rules! convert_senderror {
    ($($command:ty),*) => {
        $(
            impl From<SendError<$command>> for RLError {
                fn from(value: SendError<$command>) -> Self {
                    RLError::IO(io::Error::new(
                        io::ErrorKind::Other,
                        format!("{}: {}", stringify!($command), value),
                    ))
                }
            }
        )*
    }
}
```
Now in contrast here is a procedural macro:
```rust
#[proc_macro]
pub fn get_secrets(items: TokenStream) -> TokenStream {
    let mut result = proc_macro2::TokenStream::new();
    let items = proc_macro2::TokenStream::from(items);
    // Iterate over the tokens, extracting the identifiers.
    let mut keys = Vec::new();
    items.into_iter().for_each(|token| {
        // If the token is an identifier, add it to the list.
        if let proc_macro2::TokenTree::Ident(ident) = token {
            keys.push(ident);
        }
    });
    let secret_store = SecretStore::default();
    let secrets = keys.clone().into_iter().map(|key| {
        secret_store
            .get(key.to_string().as_str())
            .unwrap()
            .to_string()
    });
    keys.into_iter().zip(secrets).for_each(|(key, secret)| {
        result.extend(quote!(let #key = #secret;));
    });
    result.into()
}
```
In this example we are reading the identifiers from the input and loading our SecretStore, then at compile time if the
given identifiers are in the SecretStore we bind a variable with the same name to the secret. Now because we have a 
TokenStream as input and output we can normally not write Rust code directly, however we can use the `quote!` macro to 
convert normal Rust code into a TokenStream.




Of course due to them being this powerful you could also do some pretty nasty stuff with procedural Macros like write 
to disk(encrypting your files) or make network requests literally pretty much anything. 
[And while there are attempts to sandbox this](https://github.com/insanitybit/cargo-sandbox), it is why your IDE asks you if
you trust the authors of the project before expanding them. But if there is a malicious procedural macro somewhere in your 
dependency list that is a huge issue.