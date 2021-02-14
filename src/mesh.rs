use bytemuck::{Pod, Zeroable};
use nalgebra::Matrix3;

use crate::{
    flame::{BoundedState, State},
    geometry, get_state, split_levels,
    wgpu_render::SceneState,
};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Vertex {
    position: Position,
    texture_coordinate: TextureCoordinate,
}

pub type TextureCoordinate = [f32; 2];

pub type Position = [f32; 2];

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Instance {
    row0: [f32; 4],
    row1: [f32; 4],
}

pub fn build_mesh(scene: &SceneState) -> (Vec<Vertex>, Vec<Instance>) {
    let root = get_state([scene.cursor.x + 1.0, scene.cursor.y + 1.0], [2.0, 2.0]);
    let state = root.get_state();
    let bounds = state.get_bounds();
    let root_mat = geometry::letter_box(
        geometry::Rect {
            min: na::Point2::new(-1.0, -1.0),
            max: na::Point2::new(1.0, 1.0),
        },
        bounds,
    );

    let corners = bounds.corners();
    let mut positions: Vec<Position> = vec![];
    let tri_verts = [
        corners[0], corners[1], corners[2], corners[0], corners[2], corners[3],
    ];

    let uv_corners = geometry::Rect {
        min: na::Point2::new(0.0, 0.0),
        max: na::Point2::new(1.0, 1.0),
    }
    .corners();

    let mut uv_verts: Vec<TextureCoordinate> = vec![];
    let uv_tri_verts: Vec<TextureCoordinate> = [
        uv_corners[0],
        uv_corners[1],
        uv_corners[2],
        uv_corners[0],
        uv_corners[2],
        uv_corners[3],
    ]
    .iter()
    .map(|c| [c.x as f32, c.y as f32].into())
    .collect();

    let split = split_levels();

    state.process_levels(split.mesh, &mut |state| {
        for t in &tri_verts {
            let t2 = state.mat * t;
            positions.push([t2.x as f32, t2.y as f32].into());
        }
        uv_verts.extend(uv_tri_verts.iter());
    });

    let verts = positions
        .into_iter()
        .zip(uv_verts.into_iter())
        .map(|(v, u)| Vertex {
            position: v,
            texture_coordinate: u,
        })
        .collect();

    let mut instances: Vec<Instance> = vec![];
    state.process_levels(split.instance, &mut |state| {
        let m: Matrix3<f64> = (root_mat * state.mat).to_homogeneous();
        let s = m.as_slice();
        instances.push(Instance {
            row0: [s[0] as f32, s[3] as f32, s[6] as f32, 0f32],
            row1: [s[1] as f32, s[4] as f32, s[7] as f32, 0f32],
        });
    });

    return (verts, instances);
}

pub fn build_quad() -> Vec<Vertex> {
    let corners = geometry::Rect {
        min: na::Point2::new(-1.0, -1.0),
        max: na::Point2::new(1.0, 1.0),
    }
    .corners();

    let uv_corners = geometry::Rect {
        min: na::Point2::new(0.0, 0.0),
        max: na::Point2::new(1.0, 1.0),
    }
    .corners();

    [0, 1, 2, 0, 2, 3]
        .iter()
        .map(|index| -> Vertex {
            Vertex {
                position: [corners[*index].x as f32, corners[*index].y as f32].into(),
                texture_coordinate: [uv_corners[*index].x as f32, uv_corners[*index].y as f32]
                    .into(),
            }
        })
        .collect()
}
