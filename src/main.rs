mod wayland;

use crate::wayland::river_status_unstable_v1::{
    zriver_output_status_v1, zriver_seat_status_v1, zriver_status_manager_v1::ZriverStatusManagerV1,
};
use std::collections::BTreeMap;
use wayland_client::protocol::{wl_output, wl_output::WlOutput, wl_seat, wl_seat::WlSeat};
use wayland_client::{Display, GlobalManager, Main};

#[derive(Debug)]
struct Flags {
    tags: bool,
    title: bool,
    urgency: bool,
    viewstag: bool,
    output: Option<String>,
    seat: Option<String>,
}

impl Flags {
    fn default() -> Flags {
        Flags {
            tags: false,
            title: false,
            urgency: false,
            viewstag: false,
            output: None,
            seat: None,
        }
    }
}

#[derive(Debug)]
struct Env {
    flags: Flags,
    title: Option<String>,
    tags: BTreeMap<String, u32>,
    urgency: BTreeMap<String, u32>,
    viewstag: BTreeMap<String, Vec<u32>>,
    status_manager: Option<Main<ZriverStatusManagerV1>>,
}

impl Env {
    fn new() -> Env {
        Env {
            title: None,
            flags: configuration(),
            viewstag: BTreeMap::new(),
            urgency: BTreeMap::new(),
            tags: BTreeMap::new(),
            status_manager: None,
        }
    }
    fn fmt(&self) {
        if !self.tags.is_empty()
        || !self.viewstag.is_empty()
        || !self.urgency.is_empty()
        || self.title.is_some() {
            print!("{{");
            let mut comma = false;
            if !self.tags.is_empty() {
                print!("\"tags\" : [");
                let len = self.tags.len();
                for (i, (key, tags)) in self.tags.iter().enumerate() {
                    print!("{{{:?} : ", key);
                    print!("[");
                    fmt_tags(*tags);
                    print!("]}}");
                    if i < len - 1 { print!(", "); }
                }
                print!("]");
                comma = true;
            }
            if !self.urgency.is_empty() {
                print!("\"urgent\" : [");
                let len = self.urgency.len();
                for (i, (key, tags)) in self.urgency.iter().enumerate() {
                    print!("{{{:?} : ", key);
                    print!("[");
                    fmt_tags(*tags);
                    print!("]}}");
                    if i < len - 1 { print!(", "); }
                }
                print!("]");
                comma = true;
            }
            if !self.viewstag.is_empty() {
                if comma { print!(", "); }
                print!("\"viewstag\" : [");
                let vlen = self.viewstag.len();
                for (i, (key, tags)) in self.viewstag.iter().enumerate() {
                    print!("{{{:?} : ", key);
                    print!("[");
                    let len = tags.len();
                    for (i, tag) in tags.iter().enumerate() {
                        print!("\"{}\"", tag);
                        if i < len - 1 { print!(", "); }
                    }
                    print!("]}}");
                    if i < vlen - 1 { print!(", "); }
                }
                print!("]");
                comma = true;
            }
            if let Some(title) = self.title.as_ref() {
                if comma { print!(", "); }
                print!("\"title\" : {:?}", title);
            }
            println!("}}");
        }
    }
}

fn main() {
    let mut env = Env::new();

    let display = Display::connect_to_env().unwrap();
    let mut event_queue = display.create_event_queue();
    let attached_display = (*display).clone().attach(event_queue.token());

    GlobalManager::new_with_cb(
        &attached_display,
        wayland_client::global_filter!(
            [
                ZriverStatusManagerV1,
                1,
                |status_manager: Main<ZriverStatusManagerV1>, mut env: DispatchData| {
                    if let Some(env) = env.get::<Env>() {
                        env.status_manager = Some(status_manager);
                    }
                }
            ],
            [WlSeat, 7, |seat: Main<WlSeat>, _env: DispatchData| {
                seat.quick_assign(move |seat, event, mut env| match event {
                    wl_seat::Event::Name { name } => {
                        if let Some(env) = env.get::<Env>() {
                            if env.flags.title
                                && (env.flags.seat.is_none()
                                    || name.eq(env.flags.seat.as_ref().unwrap()))
                            {
                                if let Some(status_manager) = &env.status_manager {
                                    let seat_status = status_manager.get_river_seat_status(&seat);
                                    seat_status.quick_assign(
                                        move |_, event, mut env| match event {
                                            zriver_seat_status_v1::Event::FocusedView { title } => {
                                                if let Some(env) = env.get::<Env>() {
                                                    env.title = Some(title);
                                                }
                                            }
                                            _ => {}
                                        },
                                    );
                                }
                            }
                        }
                    }
                    _ => {}
                });
            }],
            [WlOutput, 3, |output: Main<WlOutput>, _env: DispatchData| {
                output.quick_assign(move |output, event, mut env| match event {
                    wl_output::Event::Geometry {
                        x: _,
                        y: _,
                        physical_width: _,
                        physical_height: _,
                        subpixel: _,
                        mut make,
                        model: _,
                        transform: _,
                    } => {
                        if let Some(env) = env.get::<Env>() {
                            if env.flags.output.is_none()
                                || env.flags.output.as_ref().unwrap().eq(&make)
                            {
                                if let Some(status_manager) = &env.status_manager {
                                    make = make.replace(' ', "").to_string();
                                    let output_status =
                                        status_manager.get_river_output_status(&output);
                                    output_status.quick_assign(move |_, event, mut env| {
                                        if let Some(env) = env.get::<Env>() {
                                            match event {
                                                zriver_output_status_v1::Event::FocusedTags {
                                                    tags,
                                                } => {
                                                    if env.flags.tags {
                                                        if let Some(inner_value) = env.tags.get_mut(&make) {
                                                            (*inner_value) = tags;
                                                        } else {
                                                            env.tags.insert(make.clone(), tags);
                                                        }
                                                    }
                                                }
                                                zriver_output_status_v1::Event::ViewTags {
                                                    tags,
                                                } => {
                                                    if env.flags.viewstag {
                                                        let tags: Vec<u32> = tags[0..]
                                                            .chunks(4)
                                                            .map(|s| {
                                                                let buf = [s[0], s[1], s[2], s[3]];
                                                                let tagmask =
                                                                    u32::from_le_bytes(buf);
                                                                for i in 0..32 {
                                                                    if 1 << i == tagmask {
                                                                        return 1+i;
                                                                    }
                                                                }
                                                                0
                                                            })
                                                            .collect();
                                                        if let Some(inner_value) = env.viewstag.get_mut(&make) {
                                                            (*inner_value) = tags;
                                                        } else {
                                                            env.viewstag.insert(make.clone(), tags);
                                                        }
                                                    }
                                                }
                                                zriver_output_status_v1::Event::UrgentTags {
                                                    tags,
                                                } => {
                                                    if env.flags.urgency {
                                                        if let Some(inner_value) = env.urgency.get_mut(&make) {
                                                            (*inner_value) = tags;
                                                        } else {
                                                            env.urgency.insert(make.clone(), tags);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    });
                                }
                            }
                        }
                    }
                    _ => {}
                });
            }]
        ),
    );

    loop {
        event_queue
            .dispatch(&mut env, |event, object, _| {
                panic!(
                    "[callop] Encountered an orphan event: {}@{}: {}",
                    event.interface,
                    object.as_ref().id(),
                    event.name
                );
            })
            .unwrap();
        env.fmt();
    }
}

fn configuration() -> Flags {
    let mut default = Flags::default();
    let mut args = std::env::args();

    loop {
        match args.next() {
            Some(flag) => match flag.as_str() {
                "--seat" 		| "-s"		=> default.seat = args.next(),
                "--output" 		| "-o"		=> default.output = args.next(),
                "--urgency" 	| "-u"		=> default.urgency = true,
                "--title" 		| "-w" 		=> default.title = true,
                "--tags" 		| "-t"		=> default.tags = true,
                "--views-tag" 	| "-vt"		=> default.viewstag = true,
                "--help"		| "-h"		=> {
                    print!("Usage: ristate [option]\n\n");
                    print!("  --tag | -t 			the focused tag\n");
                    print!("  --title | -w	   	 	the title of the focused view\n");
                    print!("  --urgency | -u 		urgent tag\n");
                    print!("  --views-tag | -vt    		the tag of all views\n");
                    print!("  --seat | -s <string>  	select the seat\n");
                    print!("  --output | -o <string> 	select the output\n");
                    std::process::exit(0);
                }
                _ => {}
            }
            None => break
        }
    }
    default
}

fn fmt_tags(tagmask: u32) {
    let mut first = true;
    for i in 0..32 {
        if tagmask >> i & 1 == 1 {
            if !first {
                print!(", \"{}\"", i + 1);
            } else {
                print!("\"{}\"", i + 1);
                first = false;
            }
        }
    }
}
