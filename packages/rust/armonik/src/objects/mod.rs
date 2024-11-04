//! The object module contains all the armonik objects for the API.
//! Each object has its own dedicated file that is re-exported here.
//! All services have their dedicated sub-modules, and rpcs have their own files within the service module which contains both a Request and Response object.
//!
//! Example:
//!
//! ```text
//! objects
//! + service1
//! | + rpc1
//! | | + Request
//! | | + Response
//! | + rpc2
//! |   + Request
//! |   + Response
//! + service2
//! | + rpc1
//! |   + Request
//! |   + Response
//! + common1
//! | + object1
//! | | + Object1
//! | + object2
//! |   + Object2
//! + object1
//!   + Object1
//! ```

mod configuration;
mod count;
mod data_chunk;
mod error;
mod filters;
mod init_keyed_data_stream;
mod init_task_request;
mod output;
mod result_request;
mod result_status;
mod session;
mod session_status;
mod sort;
mod status_count;
mod task_error;
mod task_id;
mod task_id_list;
mod task_id_with_status;
mod task_list;
mod task_options;
mod task_output_request;
mod task_request;
mod task_request_header;
mod task_status;

pub mod agent;
pub mod applications;
pub mod auth;
pub mod events;
pub mod partitions;
pub mod results;
pub mod sessions;
pub mod submitter;
pub mod tasks;
pub mod versions;
pub mod worker;

pub use configuration::Configuration;
pub use count::Count;
pub use data_chunk::DataChunk;
pub use error::Error;
pub use filters::*;
pub use init_keyed_data_stream::InitKeyedDataStream;
pub use init_task_request::InitTaskRequest;
pub use output::Output;
pub use result_request::ResultRequest;
pub use result_status::ResultStatus;
pub use session::Session;
pub use session_status::SessionStatus;
pub use sort::{Sort, SortDirection, SortMany};
pub use status_count::StatusCount;
pub use task_error::TaskError;
pub use task_id::TaskId;
pub use task_id_list::TaskIdList;
pub use task_id_with_status::TaskIdWithStatus;
pub use task_list::TaskList;
pub use task_options::{TaskOptionField, TaskOptions};
pub use task_output_request::TaskOutputRequest;
pub use task_request::TaskRequest;
pub use task_request_header::TaskRequestHeader;
pub use task_status::TaskStatus;

macro_rules! impl_convert {
    // * -> *
    (@struct {$($body:tt)*} $value:ident: $A:ty => $B:ty {$a:ident => $b:ident , $($tail:tt)*}) => {
        crate::impl_convert!(
            @struct {
                $($body)*
                $b: $value.$a.into(),
            }
            $value: $A => $B { $($tail)* }
        );
    };
    // * -> Enum *
    (@struct {$($body:tt)*} $value:ident: $A:ty => $B:ty {$a:ident => enum $b:ident , $($tail:tt)*}) => {
        crate::impl_convert!(
            @struct {
                $($body)*
                $b: $value.$a as i32,
            }
            $value: $A => $B { $($tail)* }
        );
    };
    // Enum * -> *
    (@struct {$($body:tt)*} $value:ident: $A:ty => $B:ty {enum $a:ident => $b:ident , $($tail:tt)*}) => {
        crate::impl_convert!(
            @struct {
                $($body)*
                $b: $value.$a.into(),
            }
            $value: $A => $B { $($tail)* }
        );
    };
    // * -> Option *
    (@struct {$($body:tt)*} $value:ident: $A:ty => $B:ty {$a:ident => option $b:ident , $($tail:tt)*}) => {
        crate::impl_convert!(
            @struct {
                $($body)*
                $b: Some($value.$a.into()),
            }
            $value: $A => $B { $($tail)* }
        );
    };
    // Option * -> *
    (@struct {$($body:tt)*} $value:ident: $A:ty => $B:ty {option $a:ident => $b:ident , $($tail:tt)*}) => {
        crate::impl_convert!(
            @struct {
                $($body)*
                $b: $value.$a.map_or_else(Default::default, Into::into),
            }
            $value: $A => $B { $($tail)* }
        );
    };
    // Option * -> Option *
    (@struct {$($body:tt)*} $value:ident: $A:ty => $B:ty {option $a:ident => option $b:ident , $($tail:tt)*}) => {
        crate::impl_convert!(
            @struct {
                $($body)*
                $b: $value.$a.map(Into::into),
            }
            $value: $A => $B { $($tail)* }
        );
    };
    // List * -> List *
    (@struct {$($body:tt)*} $value:ident: $A:ty => $B:ty {list $a:ident => list $b:ident , $($tail:tt)*}) => {
        crate::impl_convert!(
            @struct {
                $($body)*
                $b: crate::utils::IntoCollection::into_collect($value.$a),
            }
            $value: $A => $B { $($tail)* }
        );
    };
    // *
    (@struct {$($body:tt)*} $value:ident: $A:ty => $B:ty {$($a:ident)+ , $($tail:tt)*}) => {
        crate::impl_convert!(
            @struct {
                $($body)*
            }
            $value: $A => $B { $($a)+ => $($a)+, $($tail)* }
        );
    };
    // End of recursion
    (@struct {$($body:tt)*} $value:ident: $A:ty => $B:ty {}) => {
        impl From<$A> for $B {
            fn from($value: $A) -> Self {
                Self {
                    $($body)*
                }
            }
        }
    };
    // Entry point
    (struct $A:ty = $B:ty {$(
        $($a:ident)+ $(= $($b:ident)+)?
    ),* $(,)?}) => {
        crate::impl_convert!(@struct {} _value: $A => $B { $($($a)+ $(=> $($b)+)?,)* });
        crate::impl_convert!(@struct {} _value: $B => $A { $($($($b)+ =>)? $($a)+,)* });
        crate::impl_convert!(req $A : $B);
    };

    // Request
    (req $A:ty : $B:ty) => {
        impl tonic::IntoRequest<$B> for $A {
            fn into_request(self) -> tonic::Request<$B> {
                tonic::Request::new(self.into())
            }
        }
    };
}
pub(crate) use impl_convert;
