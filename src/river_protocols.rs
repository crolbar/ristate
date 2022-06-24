pub use generated::client::*;

pub mod generated {
    // The generated code tends to trigger a lot of warnings
    // so we isolate it into a very permissive module
    #![allow(dead_code, non_camel_case_types, unused_unsafe, unused_variables)]
    #![allow(non_upper_case_globals, non_snake_case, unused_imports)]
    #![allow(clippy::all)]

    pub mod client {
        // These imports are used by the generated code
        use wayland_client::protocol::wl_output;
        use wayland_client::{protocol, sys};
        use wayland_client::{
            AnonymousObject, Attached, Display, GlobalManager, Main, Proxy, ProxyMap,
        };
        use wayland_commons::map::{Object, ObjectMetadata};
        use wayland_commons::smallvec;
        use wayland_commons::wire::{Argument, ArgumentType, Message, MessageDesc};
        use wayland_commons::{Interface, MessageGroup};
        // pub(crate) use wayland_protocols::unstable::xdg_output::v1::client::zxdg_output_v1::Event;
        // If you protocol interacts with objects from other protocols, you'll need to import
        // their modules, like so:
        use wayland_client::protocol::{wl_region, wl_seat, wl_surface};
        include!(concat!(env!("OUT_DIR"), "/river-status-unstable-v1.rs"));
    }
}
