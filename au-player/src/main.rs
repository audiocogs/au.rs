extern crate aurora;
extern crate au;

use std::os;

fn main() {
  let (sink_0, source_0) = aurora::channel::create::<aurora::Binary>(16);
  let (sink_1, source_1) = aurora::channel::create::<aurora::Audio>(16);
  let (sink_2, source_2) = aurora::channel::create::<aurora::Binary>(16);

  let args = os::args();

  spawn(proc() {
    let path = std::path::Path::new(args[1].to_string());
    let file = std::io::File::open(&path).unwrap();
    
    aurora::file::Input::new(file, 8096, sink_0).run();
  });

  spawn(proc() {
    au::Demuxer::new(source_0, sink_1).run();
  });

  spawn(proc() {
    aurora::caf::Muxer::new(source_1, sink_2).run();
  });

  aurora::stdout::Output::new(source_2).run();
}
