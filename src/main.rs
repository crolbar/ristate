mod river_protocols;

use river_protocols::{
    zriver_output_status_v1, zriver_seat_status_v1, zriver_status_manager_v1::ZriverStatusManagerV1,
};
use serde::ser::{SerializeSeq, Serializer};
use serde::Serialize;
use std::collections::BTreeMap;
use wayland_client::protocol::{wl_output, wl_output::WlOutput, wl_seat, wl_seat::WlSeat};
use wayland_client::{Display, GlobalManager, Main};

#[derive(Debug)]
struct Flags {
    focused_tags: bool,
    focused_view: bool,
    urgency: bool,
    view_tags: bool,
    layout: bool,
    output: Option<String>,
    seat: Option<String>,
}

impl Flags {
    fn default() -> Flags {
        Flags {
            focused_tags: false,
            focused_view: false,
            urgency: false,
            view_tags: false,
            layout: false,
            output: None,
            seat: None,
        }
    }
}

struct Tags(u32);

#[derive(Serialize)]
struct Env {
    #[serde(skip)]
    flags: Flags,
    #[serde(skip_serializing_if = "Option::is_none")]
    layout: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    focused_view: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    focused_tags: Option<BTreeMap<String, Tags>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    urgency: Option<BTreeMap<String, Tags>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    view_tags: Option<BTreeMap<String, Vec<u32>>>,
    #[serde(skip)]
    status_manager: Option<Main<ZriverStatusManagerV1>>,
}

impl Env {
    fn new() -> Env {
        let flags = configuration();
        Env {
            focused_view: None,
            layout: None,
            focused_tags: flags.focused_tags.then(BTreeMap::new),
            urgency: flags.urgency.then(BTreeMap::new),
            view_tags: flags.view_tags.then(BTreeMap::new),
            status_manager: None,
            flags,
        }
    }

    fn fmt(&self) {
        if self.focused_view.is_some()
            || self.focused_tags.is_some()
            || self.urgency.is_some()
            || self.view_tags.is_some()
            || self.layout.is_some()
        {
            println!("{}", serde_json::to_string(self).unwrap());
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
                            if env.flags.focused_view
                                && (env.flags.seat.is_none()
                                    || name.eq(env.flags.seat.as_ref().unwrap()))
                            {
                                if let Some(status_manager) = &env.status_manager {
                                    let seat_status = status_manager.get_river_seat_status(&seat);
                                    seat_status.quick_assign(
                                        move |_, event, mut env| match event {
                                            zriver_seat_status_v1::Event::FocusedView { title } => {
                                                if let Some(env) = env.get::<Env>() {
                                                    env.focused_view = Some(title);
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
                                                    tags: focused_tags,
                                                } => {
                                                    if let Some(tags) = &mut env.focused_tags {
                                                        if let Some(inner_value) =
                                                            tags.get_mut(&make)
                                                        {
                                                            (*inner_value) = Tags(focused_tags);
                                                        } else {
                                                            tags.insert(
                                                                make.clone(),
                                                                Tags(focused_tags),
                                                            );
                                                        }
                                                    }
                                                }
                                                zriver_output_status_v1::Event::ViewTags {
                                                    tags,
                                                } => {
                                                    if let Some(viewstag) = &mut env.view_tags {
                                                        let tags: Vec<u32> = tags[0..]
                                                            .chunks(4)
                                                            .map(|s| {
                                                                let buf = [s[0], s[1], s[2], s[3]];
                                                                let tagmask =
                                                                    u32::from_le_bytes(buf);
                                                                for i in 0..32 {
                                                                    if 1 << i == tagmask {
                                                                        return 1 + i;
                                                                    }
                                                                }
                                                                0
                                                            })
                                                            .collect();
                                                        if let Some(inner_value) =
                                                            viewstag.get_mut(&make)
                                                        {
                                                            (*inner_value) = tags;
                                                        } else {
                                                            viewstag.insert(make.clone(), tags);
                                                        }
                                                    }
                                                }
                                                zriver_output_status_v1::Event::UrgentTags {
                                                    tags,
                                                } => {
                                                    if let Some(urgency) = &mut env.urgency {
                                                        if let Some(inner_value) =
                                                            urgency.get_mut(&make)
                                                        {
                                                            (*inner_value) = Tags(tags);
                                                        } else {
                                                            urgency
                                                                .insert(make.clone(), Tags(tags));
                                                        }
                                                    }
                                                }
                                                zriver_output_status_v1::Event::LayoutName {
                                                    name,
                                                } => {
                                                    env.layout = Some(name);
                                                }
                                                _ => {}
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
                "--seat" | "-s" => default.seat = args.next(),
                "--output" | "-o" => default.output = args.next(),
                "--urgency" | "-u" => default.urgency = true,
                "--focused-view" | "-f" => default.focused_view = true,
                "--focused-tags" | "-t" => default.focused_tags = true,
                "--layout" | "-l" => default.layout = true,
                "--view-tags" | "-vt" => default.view_tags = true,
                "--help" | "-h" => {
                    print!("Usage: ristate [option]\n\n");
                    print!("  --focused-tags | -t   the focused tag\n");
                    print!("  --focused-view | -f	the title of the focused view\n");
                    print!("  --urgency | -u        urgent tag\n");
                    print!("  --view-tags | -vt         the tag of all views\n");
                    print!("  --seat | -s <string>      select the seat\n");
                    print!("  --output | -o <string>    select the output\n");
                    print!("  --layout | -l <string>    display the name of the layout\n");
                    std::process::exit(0);
                }
                _ => {}
            },
            None => break,
        }
    }
    default
}

impl Serialize for Tags {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.0.count_ones() as usize))?;
        for i in 0..32 {
            if self.0 >> i & 1 == 1 {
                seq.serialize_element(&format!("{}", i + 1))?;
            }
        }
        seq.end()
    }
}
