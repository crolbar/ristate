mod wayland;

use crate::wayland::river_status_unstable_v1::{
    zriver_output_status_v1,
    zriver_seat_status_v1,
    zriver_status_manager_v1::ZriverStatusManagerV1,
};
use wayland_client::protocol::{
    wl_seat,
    wl_output::WlOutput,
    wl_seat::WlSeat,
};
use wayland_client::{Display, GlobalManager, Main};

struct Globals {
    seats: Vec<Main<WlSeat>>,
    outputs: Vec<Main<WlOutput>>,
    status_manager: Option<Main<ZriverStatusManagerV1>>,
}

struct Config {
    seat_name: String,
    keypair: Vec<Keypair>,
}

struct Keypair {
    key: String,
    value: String,
}

impl Keypair {
    fn to_string(&self) {
        print!(r#""{}": "{}""#, self.key, self.value)
    }
}

impl Config {
    fn mod_value(&mut self, key: String, value: String) {
        for keypair in &mut self.keypair {
            if keypair.key.eq(&key) { keypair.value = value; break }
        }
    }
    fn add_keypair(&mut self, key: String) {
        self.keypair.push({ Keypair {
            key: key,
            value: String::new()
        } });
    }
    fn to_string(&self) {
        let len = self.keypair.len();
        print!("{{");
        for (i,keypair) in self.keypair.iter().enumerate() {
            keypair.to_string();
            if i+1 < len {
                print!(", ");
            }
        }
        println!("}}");
    }
}

fn main() {
    let display = Display::connect_to_env().unwrap();

    let mut event_queue = display.create_event_queue();

    let mut globals = {
        Globals {
            seats: Vec::new(),
            outputs: Vec::new(),
            status_manager: None,
        }
    };

    let mut args = std::env::args();
    let mut config = { Config {
        seat_name: String::new(),
        keypair: Vec::new()
    } };
    let mut monitor = None;
    let mut enable_tag = false;
    let mut enable_title = false;
    let mut enable_views_tag = false;
    args.next();
    loop {
        match args.next() {
            Some(flag) => match flag.as_str() {
                "--seat" | "-s" => config.seat_name = args.next().unwrap_or(String::new()),
                "--monitor" | "-m" => {
                    monitor = match args.next().unwrap_or(String::new()).parse::<usize>() {
                        Ok(i) => Some(i),
                        Err(_) => None,
                    }
                }
                "--window-title" | "-w" => enable_title = true,
                "--tag" | "-t" => enable_tag = true,
                "--view-tags" | "-vt" => enable_views_tag = true,
                "--help" | "-h" | "--h" => {
                    println!("Usage: status [option]\n");
                    println!("  --monitor | -m <uint> : select the monitor");
                    println!("  --seat | -s <string> : select the seat");
                    println!("  --tag | -t : displays the focused tag");
                    println!("  --view-tags | -vt : displays the tag of all views");
                    println!("  --window-title | -w : displays the title of the focused view");
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
                WlSeat,
                7,
                |seat: Main<WlSeat>, mut globals: DispatchData| {
                    globals.get::<Globals>().unwrap().seats.push(seat);
                }
            ],
            [
                WlOutput,
                3,
                |output: Main<WlOutput>, mut globals: DispatchData| {
                    output.quick_assign(move |_, _, _| {});
                    globals.get::<Globals>().unwrap().outputs.push(output);
                }
            ]
        ),
    );

    event_queue
        .sync_roundtrip(&mut globals, |_, _, _| unreachable!())
        .unwrap();

    for seat in globals.seats {
        if enable_title {
            let seat_status = globals
                .status_manager
                .as_ref()
                .expect("Compositor doesn't implement river_status_unstable_v1")
                .get_river_seat_status(&seat);
            config.add_keypair("title".to_owned());
            seat.quick_assign(move |_, event, mut config| {
                let seat_name = &config.get::<Config>().unwrap().seat_name;
                match event {
                    wl_seat::Event::Name{ name } => if seat_name.len() == 0 || name.eq(seat_name) {
                        seat_status.quick_assign(move |_, event, mut config| match event {
                            zriver_seat_status_v1::Event::FocusedView { title } => {
                                config.get::<Config>().unwrap().mod_value("title".to_owned(), title);
                            },
                            _ => {}
                        })
                    } else { seat_status.quick_assign(move |_, _, _| {}) },
                    _ => {}
                }
            })
        } else { seat.quick_assign(move |_, _, _| {}) }
    }
    if enable_tag || enable_views_tag {
        for (i, output) in globals
            .outputs
            .iter()
            .enumerate()
            .filter(|(i, _)| if let Some(index) = monitor {
                if *i == index { true } else { false }
            } else { true })
        {
            if enable_tag { config.add_keypair(format!("tag{}",i)); }
            if enable_views_tag { config.add_keypair(format!("views_tag{}",i)); }
            let output_status = globals
                .status_manager
                .as_ref()
                .expect("Compositor doesn't implement river_status_unstable_v1")
                .get_river_output_status(&output);
            output_status.quick_assign(move |_, event, mut config| match event {
                zriver_output_status_v1::Event::FocusedTags { tags } => {
                    if enable_tag {
                        config.get::<Config>().unwrap().mod_value(format!("tag{}",i), base10(tags).trim_end().to_owned());
                    }
                }
                zriver_output_status_v1::Event::ViewTags { tags } => {
                    if enable_views_tag {
                        let len = tags.len();
                        let mut views_tag = String::new();
                        for i in (0..len).into_iter().step_by(4) {
                            let buf: [u8; 4] = [tags[i], tags[i + 1], tags[i + 2], tags[i + 3]];
                            views_tag.push_str(&base10(u32::from_le_bytes(buf)));
                        }
                        config.get::<Config>().unwrap().mod_value(format!("views_tag{}",i), views_tag.trim_end().to_owned());
                    }
                }
            });
        }
    }

    loop {
        event_queue
            .dispatch(&mut config, |event, object, _| {
                panic!(
                    "[callop] Encountered an orphan event: {}@{}: {}",
                    event.interface,
                    object.as_ref().id(),
                    event.name
                );
            })
            .unwrap();
        if enable_views_tag || enable_tag || enable_title { config.to_string(); }
    }
}

fn base10(tagmask: u32) -> String {
    let mut format = String::new();
    let mut tag = 0;
    let mut current: u32;
    while {
        current = 1 << tag;
        current <= tagmask
    } {
        tag += 1;
        if current != tagmask && (tagmask / current) % 2 != 0 {
            format.push_str(&(base10(tagmask - current)));
            break;
        } else if tag == 32 { break; }
    }
    format.push_str(&tag.to_string());
    format.push(' ');
    format
}
