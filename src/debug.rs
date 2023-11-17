/// this module contains a debug implementation of the cabac reader and writer. It is used to verify that the
/// correct context is always passed into the get and put functions. If the correct index is not passed, it
/// can lead to very subtle consistency bugs, so it is worthwhile to test with the debug implementation.
use std::io::{Read, Result, Write};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use crate::traits::{CabacReader, CabacWriter};

// we make sure that every context has a different value so that we can detect when we've made
// a mistake and are using the wrong context
pub struct DebugContext {
    value: u32,
}

impl Default for DebugContext {
    /// default value is balanced between zeros or ones
    fn default() -> Self {
        DebugContext { value: 0 }
    }
}

pub struct DebugReader<R> {
    reader: R,
    counter: u32,
}

impl<R: Read> DebugReader<R> {
    pub fn new(reader: R) -> Result<Self> {
        Ok(DebugReader {
            reader: reader,
            counter: 100,
        })
    }
}

impl<R: Read> CabacReader<DebugContext> for DebugReader<R> {
    /// reads a single 1 or 0 from the bitstream using the probability of the supplied context
    fn get(&mut self, branch: &mut DebugContext) -> Result<bool> {
        if branch.value == 0 {
            self.counter += 1;
            branch.value = self.counter;
        }

        assert_eq!(branch.value, self.reader.read_u32::<LittleEndian>()?);
        self.counter += 1;
        branch.value = self.counter;
        Ok(self.reader.read_u8()? != 0)
    }

    /// reads a single 1 or 0 from the bitstream using a fixed probabilty of 0.5
    /// this results in a faster logic for bits where the probability is close to 0.5 and
    /// compression is not worthwhile.
    fn get_bypass(&mut self) -> Result<bool> {
        assert_eq!(0xdead, self.reader.read_u32::<LittleEndian>()?);
        Ok(self.reader.read_u8()? != 0)
    }
}

/// encoder from VP8/WebM
pub struct DebugWriter<W> {
    writer: W,
    counter: u32,
}

impl<W: Write> DebugWriter<W> {
    pub fn new(writer: W) -> Result<Self> {
        Ok(DebugWriter {
            writer: writer,
            counter: 100,
        })
    }
}

impl<W: Write> CabacWriter<DebugContext> for DebugWriter<W> {
    fn put(&mut self, value: bool, branch: &mut DebugContext) -> Result<()> {
        if branch.value == 0 {
            self.counter += 1;
            branch.value = self.counter;
        }

        self.writer.write_u32::<LittleEndian>(branch.value)?;
        self.counter += 1;
        branch.value = self.counter;

        self.writer.write_u8(value as u8)?;
        Ok(())
    }

    fn finish(&mut self) -> Result<()> {
        Ok(())
    }

    fn put_bypass(&mut self, value: bool) -> Result<()> {
        self.writer.write_u32::<LittleEndian>(0xdead)?;
        self.writer.write_u8(value as u8)?;
        Ok(())
    }
}

#[test]
fn roundtrip_value() {
    let mut output = Vec::with_capacity(1000);
    {
        let mut writer = DebugWriter::new(&mut output).unwrap();
        let mut context = [
            DebugContext::default(),
            DebugContext::default(),
            DebugContext::default(),
            DebugContext::default(),
        ];
        for i in 0..100 {
            writer.put(i & 1 == 1, &mut context[i % 4]).unwrap();
            writer.put_bypass(i & 1 == 1).unwrap();
        }

        writer.finish().unwrap();
    }

    {
        let mut reader = DebugReader::new(&output[..]).unwrap();
        let mut context = [
            DebugContext::default(),
            DebugContext::default(),
            DebugContext::default(),
            DebugContext::default(),
        ];
        for i in 0..100 {
            assert_eq!(reader.get(&mut context[i % 4]).unwrap(), i & 1 == 1);
            assert_eq!(reader.get_bypass().unwrap(), i & 1 == 1);
        }
    }
}
