use std::path::Path;

use super::TerrainChunk;
use bevy::prelude::{Color, Vec3};
use image::{io::Reader, ImageBuffer, ImageResult, Luma, Rgba};

impl TerrainChunk {
    #[allow(unused)]
    pub fn from_images(
        height: impl AsRef<Path>,
        color: impl AsRef<Path>,
        chunk_size: usize,
        hscale: f32,
        vscale: f32,
    ) -> ImageResult<impl Iterator<Item = Self>> {
        log::info!("Generating chunks from images");
        let height = Reader::open(height)?.decode()?.into_luma16();
        let color = Reader::open(color)?.decode()?.into_rgba32f();
        let w = height.width();
        let h = height.height();
        struct Iter<I> {
            iter: I,
            height: ImageBuffer<Luma<u16>, Vec<u16>>,
            color: ImageBuffer<Rgba<f32>, Vec<f32>>,
            chunk_size: usize,
            hscale: f32,
            vscale: f32,
        }
        impl<T: Iterator<Item = (u32, u32)>> Iterator for Iter<T> {
            type Item = TerrainChunk;
            fn next(&mut self) -> Option<Self::Item> {
                let (x, z) = self.iter.next()?;
                Some(TerrainChunk::gen_function(
                    x,
                    z,
                    &mut self.height,
                    &mut self.color,
                    self.chunk_size,
                    self.hscale,
                    self.vscale,
                ))
            }
        }
        Ok(Iter {
            iter: (0..w)
                .step_by(chunk_size)
                .flat_map(move |x| (0..h).step_by(chunk_size).map(move |z| (x, z))),
            height,
            color,
            chunk_size,
            hscale,
            vscale,
        })

        // log::info!("Generated {} chunks", chunks.len());
    }

    fn gen_function(
        x: u32,
        z: u32,
        height: &mut ImageBuffer<Luma<u16>, Vec<u16>>,
        color: &mut ImageBuffer<Rgba<f32>, Vec<f32>>,
        chunk_size: usize,
        hscale: f32,
        vscale: f32,
    ) -> Self {
        log::debug!("Generating chunk {:?}", (x, z));
        let Luma([h]) = *height.get_pixel(x, z);
        let mut min = h as f32 * vscale;
        let mut max = h as f32 * vscale;
        let mut heights = vec![];
        let mut colors = vec![];
        for z_off in 0..chunk_size as u32 + 1 {
            for x_off in 0..chunk_size as u32 + 1 {
                let (h, r, g, b, a) = if x + x_off < height.width() && z + z_off < height.height() {
                    let Luma([h]) = *height.get_pixel(x + x_off, z + z_off);
                    let Rgba([r, g, b, a]) = *color.get_pixel(x + x_off, z + z_off);
                    (h, r, g, b, a)
                } else {
                    // continue;
                    (0, 1., 1., 1., 1.)
                };
                heights.push(h as f32 * vscale);
                colors.push(Color::rgba(r, g, b, a));
                min = min.min(h as f32 * vscale);
                max = max.min(h as f32 * vscale);
            }
        }
        let correction = (max - min) / 2. + min;
        for h in &mut heights {
            *h -= correction;
        }
        Self {
            size: chunk_size + 1,
            points: heights,
            color: colors,
            min: Vec3::new(x as f32, min, z as f32) * hscale,
            max: Vec3::new(
                x as f32 + chunk_size as f32,
                max,
                z as f32 + chunk_size as f32,
            ) * hscale,
            mesh: None,
        }
    }
}
