mod wayland;

use wayland_client::protocol::wl_output::WlOutput;
use crate::wayland::{
    river_status_unstable_v1::zriver_status_manager_v1::ZriverStatusManagerV1,
    river_status_unstable_v1::zriver_output_status_v1::ZriverOutputStatusV1,
};
use crate::wayland::river_status_unstable_v1::zriver_output_status_v1;
use wayland_client::{Display, GlobalManager, Main};

struct Globals {
    outputs: Vec<WlOutput>,
    status_manager: Option<Main<ZriverStatusManagerV1>>
}

fn main() {
    let display = Display::connect_to_env().unwrap();

    let mut event_queue = display.create_event_queue();

    let mut globals = { Globals {
        outputs: Vec::new(),
        status_manager: None
    } };

    let mut args = std::env::args();
    let mut monitor = None;
    let mut enable_tag = false;
    let mut enable_views_tag = false;
    args.next();
    loop {
        match args.next() {
            Some(flag) => match flag.as_str() {
                "--monitor" | "-m" => monitor = match args.next().unwrap_or(String::new()).parse::<usize>() {
                    Ok(i) => Some(i),
                    Err(_) => None,
                },
                "--tag" | "-t" => enable_tag = true,
                "--view-tags" | "-vt" => enable_views_tag = true,
                "--help" | "-h" | "--h" => {
                    println!("Usage: status [option]\n");
                    println!("  --tag | -t : displays the focused tag");
                    println!("  --view-tags | -vt : displays the tag of all views");
                    println!("  --monitor | -m : select the monitor index");
                    std::process::exit(0);
                }
                _ => break,
            },
            None => break,
        }
    }

    let attached_display = (*display).clone().attach(event_queue.token());

    let _ = GlobalManager::new_with_cb(
        &attached_display,
        wayland_client::global_filter!(
            [
                ZriverStatusManagerV1,
                1,
                |status_manager_obj: Main<ZriverStatusManagerV1>, mut globals: DispatchData| {
                    globals.get::<Globals>().unwrap().status_manager = Some(status_manager_obj);
                }
            ],
            [
                WlOutput,
                3,
                |output: Main<WlOutput>, mut globals: DispatchData| {
                    output.quick_assign(move |_, _, _| {});
                    globals.get::<Globals>().unwrap().outputs.push(output.detach());
                }
            ]
        ),
    );

    event_queue
        .sync_roundtrip(&mut globals, |_, _, _| unreachable!())
        .unwrap();

    for (i, output) in globals.outputs.iter_mut().enumerate() {
        let assign;
        match monitor {
            Some(monitor) => if i == monitor {
                assign = true;
            } else { assign = false },
            None => assign = true
        }
        if assign {
            let output_status = globals.status_manager
                .as_ref()
                .expect("Compositor doesn't implement river_status_unstable_v1")
                .get_river_output_status(&output);
            output_status.quick_assign(move |_, event, _| match event {
                zriver_output_status_v1::Event::FocusedTags { tags } => {
                    if enable_tag {
                        base10(tags);
                        println!("");
                    }
                }
                zriver_output_status_v1::Event::ViewTags { tags } => {
                    if enable_views_tag {
                        let len = tags.len();
                        let mut i = 0;
                        while i < len {
                            let buf: [u8; 4] = [tags[i],tags[i+1],tags[i+2],tags[i+3]];
                            base10(u32::from_le_bytes(buf));
                            i+=4;
                        }
                        println!("");
                    }
                }
            });
        }
    }

    loop {
        event_queue
            .dispatch(&mut (), |event, object, _| {
                panic!(
                    "[callop] Encountered an orphan event: {}@{}: {}",
                    event.interface,
                    object.as_ref().id(),
                    event.name
                );
            })
            .unwrap();
    }
}

fn base10(tagmask: u32) {
    let mut tag = 0;
    let mut current: u32;
    while {current = 1 << tag; current <= tagmask} {
        tag += 1;
        if current != tagmask && (tagmask/current) % 2 != 0 {
            base10(tagmask-current);
            break;
        } else if tag == 32 { break }
    }
    print!("{} ", tag);
}
