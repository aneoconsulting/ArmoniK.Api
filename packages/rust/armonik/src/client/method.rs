//! [`client_method!`]: one client convenience method, from one line.
//!
//! Most client methods are the same shape: widen a few arguments, build the request, call it, and
//! hand back one field of the response. This writes that shape; anything else is written out as an
//! ordinary `fn` beside it.
//!
//! What it does *not* do is decide the signature. The parameters, their order and their types are
//! all spelled at the call site, which is the point: a signature that is written down cannot move
//! when a field is added to the proto message behind it. The classes below only say how an argument
//! is widened and converted back, which is a property of the parameter rather than of the schema.

/// How one argument is widened in the signature. Paired with [`param_value`], which converts it
/// back; the two must agree, so they are written next to each other and read as one table.
///
/// A macro rather than a branch inside `client_method!`'s repetition, because `macro_rules!` cannot
/// vary its expansion per element of a repetition. Both positions accept a macro call, including
/// argument-position `impl Trait`.
// Which client methods exist depends on which use-case feature is on, and the smallest surfaces use
// none of these: `--features agent` compiles only `client/worker.rs`, whose single method takes no
// arguments at all. So the helpers below are allowed to go unused, per feature configuration rather
// than per build.
#[allow(unused_macros)]
macro_rules! param_ty {
    (into $t:ty) => { impl ::core::convert::Into<$t> };
    (iter $t:ty) => {
        impl ::core::iter::IntoIterator<Item = impl ::core::convert::Into<$t>>
    };
    (pairs $k:ty, $v:ty) => {
        impl ::core::iter::IntoIterator<
            Item = (impl ::core::convert::Into<$k>, impl ::core::convert::Into<$v>),
        >
    };
    (filters $t:ty) => {
        impl ::core::iter::IntoIterator<Item = impl ::core::iter::IntoIterator<Item = $t>>
    };
    (plain $t:ty) => { $t };
}

/// The conversion back to the field's own type, paired with [`param_ty`].
#[allow(unused_macros)]
macro_rules! param_value {
    (into $t:ty, $n:ident) => {
        $n.into()
    };
    (iter $t:ty, $n:ident) => {
        crate::utils::IntoCollection::into_collect($n)
    };
    (pairs $k:ty, $v:ty, $n:ident) => {
        $n.into_iter()
            .map(|(key, value)| (key.into(), value.into()))
            .collect()
    };
    (filters $t:ty, $n:ident) => {
        crate::utils::into_filters($n)
    };
    (plain $t:ty, $n:ident) => {
        $n
    };
}

/// One client convenience method.
///
/// ```ignore
/// client_method!(GetSession: get(session_id: into<String>) -> get::Request => session: Raw);
/// client_method!(ListSessions: list(page: plain<i32>) -> list::Request => list::Response);
/// client_method!(CancelTasks: cancel(filter: plain<Filter>) -> cancel::Request => ());
/// client_method!(DownloadResultData:
///     download(result_id: into<String>) -> stream download::Request => data_chunk: Bytes);
/// ```
///
/// Attributes written before the RPC name are passed through to the method, which is how the
/// `Submitter` methods carry their `#[deprecated]`.
///
/// The leading name is the RPC this method stands for. `#[armonik_macros::client]` reads it to
/// prepend the RPC's documentation and to register the method for the coverage check, and does so by
/// prepending an `@docs { ... }` block; it parses nothing after that name, which is what keeps the
/// two macros independent of each other. The `@` is load-bearing: a bare `docs` is a valid ident, so
/// the optional group would be ambiguous with the RPC name that follows it.
macro_rules! client_method {
    // Server-streaming, projected to one field of each item.
    (
        $(@docs { $(#[$doc:meta])* })?
        $(#[$extra:meta])*
        $rpc:ident: $name:ident ( $($p:ident : $cls:ident < $($t:ty),+ >),* $(,)? )
            -> stream $($req:ident)::+ => $field:ident : $item:ty
    ) => {
        $($(#[$doc])*)?
        $(#[$extra])*
        pub async fn $name(
            &mut self,
            $($p: crate::client::method::param_ty!($cls $($t),+)),*
        ) -> ::core::result::Result<
            ::futures::stream::BoxStream<
                'static,
                ::core::result::Result<$item, crate::client::RequestError>,
            >,
            crate::client::RequestError,
        > {
            ::core::result::Result::Ok(::futures::StreamExt::boxed(::futures::StreamExt::map(
                self.call($($req)::+ { $($p: crate::client::method::param_value!($cls $($t),+, $p)),* }).await?,
                |item| item.map(|response| response.$field),
            )))
        }
    };

    // Server-streaming, whole items.
    (
        $(@docs { $(#[$doc:meta])* })?
        $(#[$extra:meta])*
        $rpc:ident: $name:ident ( $($p:ident : $cls:ident < $($t:ty),+ >),* $(,)? )
            -> stream $($req:ident)::+ => $item:ty
    ) => {
        $($(#[$doc])*)?
        $(#[$extra])*
        pub async fn $name(
            &mut self,
            $($p: crate::client::method::param_ty!($cls $($t),+)),*
        ) -> ::core::result::Result<
            ::futures::stream::BoxStream<
                'static,
                ::core::result::Result<$item, crate::client::RequestError>,
            >,
            crate::client::RequestError,
        > {
            self.call($($req)::+ { $($p: crate::client::method::param_value!($cls $($t),+, $p)),* }).await
        }
    };

    // Unary, response discarded. Before the whole-response rule, which would also match `()`.
    (
        $(@docs { $(#[$doc:meta])* })?
        $(#[$extra:meta])*
        $rpc:ident: $name:ident ( $($p:ident : $cls:ident < $($t:ty),+ >),* $(,)? )
            -> $($req:ident)::+ => ()
    ) => {
        $($(#[$doc])*)?
        $(#[$extra])*
        pub async fn $name(
            &mut self,
            $($p: crate::client::method::param_ty!($cls $($t),+)),*
        ) -> ::core::result::Result<(), crate::client::RequestError> {
            self.call($($req)::+ { $($p: crate::client::method::param_value!($cls $($t),+, $p)),* }).await?;
            ::core::result::Result::Ok(())
        }
    };

    // Unary, projected to one field. Before the whole-response rule, whose `$ret:ty` would swallow
    // the field name.
    (
        $(@docs { $(#[$doc:meta])* })?
        $(#[$extra:meta])*
        $rpc:ident: $name:ident ( $($p:ident : $cls:ident < $($t:ty),+ >),* $(,)? )
            -> $($req:ident)::+ => $field:ident : $ret:ty
    ) => {
        $($(#[$doc])*)?
        $(#[$extra])*
        pub async fn $name(
            &mut self,
            $($p: crate::client::method::param_ty!($cls $($t),+)),*
        ) -> ::core::result::Result<$ret, crate::client::RequestError> {
            ::core::result::Result::Ok(
                self.call($($req)::+ { $($p: crate::client::method::param_value!($cls $($t),+, $p)),* }).await?.$field,
            )
        }
    };

    // Unary, whole response.
    (
        $(@docs { $(#[$doc:meta])* })?
        $(#[$extra:meta])*
        $rpc:ident: $name:ident ( $($p:ident : $cls:ident < $($t:ty),+ >),* $(,)? )
            -> $($req:ident)::+ => $ret:ty
    ) => {
        $($(#[$doc])*)?
        $(#[$extra])*
        pub async fn $name(
            &mut self,
            $($p: crate::client::method::param_ty!($cls $($t),+)),*
        ) -> ::core::result::Result<$ret, crate::client::RequestError> {
            self.call($($req)::+ { $($p: crate::client::method::param_value!($cls $($t),+, $p)),* }).await
        }
    };
}

#[allow(unused_imports)]
pub(crate) use {client_method, param_ty, param_value};

#[cfg(test)]
mod tests {
    //! What the widening table accepts, pinned directly.
    //!
    //! Compile-only, because the widening *is* the behaviour: a function that accepts the argument
    //! proves the signature, and calling it would only re-test `ServiceClient::call`. The
    //! `rpc_tests!` suites drive every one of these classes through a real method already, and pin
    //! `into`, both levels of `iter` and both of `pairs` by passing `&str` and `&[u8]` where the
    //! field wants `String` and `Bytes`. What they do not pin is `filters`: every call site passes a
    //! `filter::Or` straight through, which would still compile if the class were `plain`.

    fn into(_: crate::client::method::param_ty!(into String)) {}
    fn iter(_: crate::client::method::param_ty!(iter String)) {}
    fn pairs(_: crate::client::method::param_ty!(pairs String, bytes::Bytes)) {}
    fn filters(_: crate::client::method::param_ty!(filters crate::sessions::filter::Field)) {}
    fn plain(_: crate::client::method::param_ty!(plain i32)) {}

    #[test]
    fn each_class_widens_its_argument() {
        into("a borrowed str, where the field holds a String");
        iter(["items convertible to the element type"]);
        pairs([("key", b"value".as_slice())]);
        // Two levels, and neither is a `filter::Or`: this is what a `plain` class would reject.
        filters([[crate::sessions::filter::Field::default()]]);
        plain(7);
    }
}
