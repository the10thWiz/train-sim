pub mod chunked;
pub mod from_map;

use std::io::{self, Cursor, Write};

use crate::util::{ReadBytesExt2, WriteBytesExt2};
use bevy::{
    asset::{AssetLoader, Error, LoadContext, LoadedAsset},
    prelude::*,
    reflect::TypeUuid,
    render::{mesh::Indices, render_resource::PrimitiveTopology},
    utils::BoxedFuture,
};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use self::chunked::ChunkedLoader;

pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.add_asset::<TerrainChunk>();
        app.add_asset_loader(TerrainLoader {});
        let loader = ChunkedLoader::new(app);
        app.insert_resource(loader);
        app.add_system(ChunkedLoader::update);
    }
}

#[derive(Debug, Clone, TypeUuid)]
#[uuid = "396a2da0-cb69-44d5-9ae3-94d75beba559"]
pub struct TerrainChunk {
    size: usize,
    points: Vec<f32>,
    color: Vec<Color>,
    max: Vec3,
    min: Vec3,
    mesh: Option<Handle<Mesh>>,
}

impl TerrainChunk {
    #[allow(unused)]
    pub fn generate_height_map(
        sx: f32,
        sz: f32,
        size: usize,
        mut height: impl FnMut(f32, f32) -> f32,
        pos: Vec3,
    ) -> Self {
        let mut min = Vec3::new(sx * -0.5, 0., sz * -0.5) + pos;
        let mut max = Vec3::new(sx * 0.5, 0., sz * 0.5) + pos;
        let points = Self::points(size, min, max)
            .map(|(_i, x, z)| {
                let y = height(x + pos.x, z + pos.z);
                min.y = min.y.min(y);
                max.y = max.y.max(y);
                y
            })
            .collect();
        Self {
            size,
            points,
            color: vec![Color::BLACK; size * size],
            min,
            max,
            mesh: None,
        }
    }

    fn points(size: usize, min: Vec3, max: Vec3) -> impl Iterator<Item = (usize, f32, f32)> {
        let dist = max - min;
        let mut step = dist;
        step.x /= (size - 1) as f32;
        step.z /= (size - 1) as f32;
        (0..size * size).map(move |i| {
            (
                i,
                step.x * (i % size) as f32 - dist.x / 2.,
                step.z * (i / size) as f32 - dist.z / 2.,
            )
        })
    }

    fn generate_mesh(&self) -> Mesh {
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_POSITION,
            Self::points(self.size, self.min, self.max)
                .map(|(i, x, z)| Vec3::new(x, self.points[i], z))
                .collect::<Vec<_>>(),
        );
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_NORMAL,
            self.points
                .iter()
                .map(|_| Vec3::new(0., 1., 0.))
                .collect::<Vec<_>>(),
        );
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_COLOR,
            self.color
                .iter()
                .map(|c| c.as_linear_rgba_f32())
                .collect::<Vec<_>>(),
        );
        let mut indicies = Vec::with_capacity((self.size - 1) * (self.size - 1) * 6);
        for x in 0..self.size - 1 {
            for z in 0..self.size - 1 {
                indicies.push(self.idx(x, z) as u32);
                indicies.push(self.idx(x, z + 1) as u32);
                indicies.push(self.idx(x + 1, z) as u32);

                indicies.push(self.idx(x, z + 1) as u32);
                indicies.push(self.idx(x + 1, z + 1) as u32);
                indicies.push(self.idx(x + 1, z) as u32);
            }
        }
        mesh.set_indices(Some(Indices::U32(indicies)));
        mesh
    }

    pub fn move_mesh(&mut self, offset: Vec3) {
        self.min += offset;
        self.max += offset;
    }

    fn idx(&self, x: usize, z: usize) -> usize {
        // debug_assert!(x <= self.width as usize && z <= self.points.len() / self.width as usize);
        x + self.size as usize * z
    }

    pub fn mesh(&self) -> Handle<Mesh> {
        self.mesh.clone().unwrap()
    }

    pub fn pos(&self) -> Vec3 {
        (self.min + self.max) / 2.
    }

    #[allow(unused)]
    pub fn contains(&self, pos: Vec3) -> bool {
        self.min.x <= pos.x
            && self.min.y <= pos.y
            && self.min.z <= pos.z
            && self.max.x >= pos.x
            && self.max.y >= pos.y
            && self.max.z >= pos.z
    }
}

#[derive(Debug, Clone, TypeUuid)]
#[uuid = "9855520e-351b-4c89-9385-5ead3209f68a"]
pub struct TerrainLoader {}

type LE = LittleEndian;

// #[derive(Debug, Error)]
// enum LoadError {
//     #[error("Height is out of bounds")]
//     HeightOutOfBounds,
// }

impl TerrainLoader {
    pub fn load_chunk<'a>(&'a self, bytes: &'a [u8]) -> Result<TerrainChunk, Error> {
        let mut reader = Cursor::new(bytes);
        let min = reader.read_vec3::<LE>()?;
        let max = reader.read_vec3::<LE>()?;
        let size = reader.read_u64::<LE>()? as usize;
        let mut points = Vec::with_capacity(size * size);
        let mut min_y: f32 = 0.;
        let mut max_y: f32 = 0.;
        for _ in 0..size * size {
            let p = reader.read_f32::<LE>()?;
            points.push(p);
            min_y = min_y.min(p);
            max_y = max_y.max(p);
            // if min.y > p || max.y < p {
            //     return Err(LoadError::HeightOutOfBounds.into());
            // }
        }
        log::info!("[{} .. {}]", min_y, max_y);
        let mut color = Vec::with_capacity(size * size);
        for _ in 0..size * size {
            let [r, g, b, a] = reader.read_u32::<LE>()?.to_le_bytes();
            color.push(Color::rgba_u8(r, g, b, a));
        }
        Ok(TerrainChunk {
            size,
            points,
            color,
            min,
            max,
            mesh: None,
        })
    }

    pub fn write_chunk<'a>(
        &'a self,
        mut writer: impl Write,
        chunk: &TerrainChunk,
    ) -> io::Result<()> {
        writer.write_vec3::<LE>(chunk.min)?;
        writer.write_vec3::<LE>(chunk.max)?;
        writer.write_u64::<LE>(chunk.size as u64)?;
        for p in &chunk.points {
            writer.write_f32::<LE>(*p)?;
        }
        for c in &chunk.color {
            writer.write_u32::<LE>(c.as_linear_rgba_u32())?;
        }
        Ok(())
    }
}

impl AssetLoader for TerrainLoader {
    fn extensions(&self) -> &[&str] {
        &["chunk"]
    }

    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<(), Error>> {
        let res = self.load_chunk(bytes).map(|mut chunk| {
            chunk.mesh = Some(
                load_context.set_labeled_asset("mesh", LoadedAsset::new(chunk.generate_mesh())),
            );
            load_context.set_default_asset(LoadedAsset::new(chunk))
        });
        Box::pin(async move { res })
    }
}
