use std::io;

use bevy::prelude::*;
use byteorder::{ByteOrder, ReadBytesExt, WriteBytesExt};

pub trait ReadBytesExt2: ReadBytesExt {
    fn read_vec2<T: ByteOrder>(&mut self) -> io::Result<Vec2> {
        Ok(Vec2::new(self.read_f32::<T>()?, self.read_f32::<T>()?))
    }

    fn read_vec3<T: ByteOrder>(&mut self) -> io::Result<Vec3> {
        Ok(Vec3::new(
            self.read_f32::<T>()?,
            self.read_f32::<T>()?,
            self.read_f32::<T>()?,
        ))
    }
}

impl<T: ReadBytesExt> ReadBytesExt2 for T {}

pub trait WriteBytesExt2: WriteBytesExt {
    fn write_vec2<T: ByteOrder>(&mut self, v: Vec3) -> io::Result<()> {
        self.write_f32::<T>(v.x)?;
        self.write_f32::<T>(v.y)?;
        Ok(())
    }

    fn write_vec3<T: ByteOrder>(&mut self, v: Vec3) -> io::Result<()> {
        self.write_f32::<T>(v.x)?;
        self.write_f32::<T>(v.y)?;
        self.write_f32::<T>(v.z)?;
        Ok(())
    }
}

impl<T: WriteBytesExt> WriteBytesExt2 for T {}
