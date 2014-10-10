extern crate aurora;

use aurora::sample_type::{Unknown,Unsigned,Float};

pub struct Demuxer {
  source: aurora::channel::Source<aurora::Binary>,
  sink: aurora::channel::Sink<aurora::Audio>
}

impl Demuxer {
  pub fn new(source: aurora::channel::Source<aurora::Binary>, sink: aurora::channel::Sink<aurora::Audio>) -> Demuxer {
    return Demuxer {
      source: source,
      sink: sink
    };
  }

  pub fn run(&mut self) {
    let mut stream = aurora::stream::Stream::new(&mut self.source);

    let mut fourcc = [0x00, ..4];

    stream.read(fourcc);

    if fourcc != b".snd" {
      fail!("au::Demuxer: Stream did not start with fourcc '.snd' had bytes {:x}{:x}{:x}{:x} (INPUT)", fourcc[0], fourcc[1], fourcc[2], fourcc[3]);
    }

    let data_offset = stream.read_be_u32();
    let data_size = stream.read_be_u32();

    let sample_type = match stream.read_be_u32() {
      2 => Unsigned(8),
      3 => Unsigned(16),
      4 => Unsigned(24),
      5 => Unsigned(32),
      6 => Float(32),
      7 => Float(64),
      _ => Unknown
    };

    let sample_rate = stream.read_be_u32() as f64;
    let channels = stream.read_be_u32() as uint;

    stream.skip(data_offset as uint - 24); // Jump to the data.

    let mut final = false;
    let mut remaining_data = data_size as uint;

    let sample_size = aurora::sample_type::size(sample_type);
    let chunk_size = (sample_size * channels / 8) * 1024; // Random number

    while !final {
      self.sink.write(|audio| {
        audio.data.grow(std::cmp::min(chunk_size, remaining_data), 0);

        stream.read(audio.data.as_mut_slice()); // If size is unknown, then this will fail

        remaining_data -= audio.data.len();
        final = remaining_data == 0;

        audio.final = final;
        audio.channels = channels;
        audio.sample_rate = sample_rate;
        audio.endian = aurora::endian::Big;
        audio.sample_type = sample_type;
      });
    }
  }
}

pub struct Muxer {
  source: aurora::channel::Source<aurora::Audio>,
  sink: aurora::channel::Sink<aurora::Binary>
}

impl Muxer {
  pub fn new(source: aurora::channel::Source<aurora::Audio>, sink: aurora::channel::Sink<aurora::Binary>) -> Muxer {
    return Muxer {
      source: source,
      sink: sink
    };
  }

  pub fn run(&mut self) {
    let mut first = true;
    let mut final = false;

    let source = &mut self.source;
    let sink = &mut self.sink;

    while !final {
      source.read(|audio| {
        if first {
          sink.write(|binary| {
            let d = &mut binary.data;

            d.grow(24, 0);

            std::slice::bytes::copy_memory(d.slice_mut( 0,  4), b".snd");

            let sample_type: u32 = match audio.sample_type {
              Unsigned(8) => 2,
              Unsigned(16) => 3,
              Unsigned(24) => 4,
              Unsigned(32) => 5,
              Float(32) => 6,
              Float(64) => 7,
              _ => fail!("au::Muxer: Unsupported sample type {} (INPUT)", audio.sample_type)
            };

            unsafe {
              std::slice::bytes::copy_memory(d.slice_mut( 4,  8), std::mem::transmute::<u32, [u8, .. 4]>(24u32.to_be()));
              std::slice::bytes::copy_memory(d.slice_mut( 8, 12), std::mem::transmute::<u32, [u8, .. 4]>(0xFFFFFFFF));
              std::slice::bytes::copy_memory(d.slice_mut(12, 16), std::mem::transmute::<u32, [u8, .. 4]>(sample_type.to_be()));
              std::slice::bytes::copy_memory(d.slice_mut(16, 20), std::mem::transmute::<u32, [u8, .. 4]>((audio.sample_rate as u32).to_be()));
              std::slice::bytes::copy_memory(d.slice_mut(20, 24), std::mem::transmute::<u32, [u8, .. 4]>((audio.channels as u32).to_be()));
            }

            first = false;
          });
        }

        final = audio.final;

        sink.write(|binary| {
          binary.data.grow(audio.data.len(), 0);

          std::slice::bytes::copy_memory(binary.data.as_mut_slice(), audio.data.as_slice());

          binary.final = final;
        });
      });
    }
  }
}

#[cfg(test)]
mod tests {
  use std;
  use aurora;

  #[test]
  fn test_float32() {
    let (sink_0, source_0) = aurora::channel::create::<aurora::Binary>(1);
    let (sink_1, mut source_1) = aurora::channel::create::<aurora::Audio>(1);

    spawn(proc() {
      let path = std::path::Path::new("./test-vectors/M1F1-float32-AFsp.au");
      let file = std::io::File::open(&path).unwrap();

      aurora::file::File::new(file, 4096, sink_0).run();
    });

    spawn(proc() {
      super::Demuxer::new(source_0, sink_1).run();
    });

    source_1.read(|audio| {
      assert_eq!(audio.final, false);
      assert_eq!(audio.channels, 2);
      assert_eq!(audio.sample_rate, 8000.0);
      assert_eq!(audio.endian, aurora::endian::Big);
      assert_eq!(audio.sample_type, aurora::sample_type::Float(32));
    });
  }
}
