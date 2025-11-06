// use bevy::asset::RenderAssetUsages;
// use bevy::math::{FloatPow, Vec3};
// use bevy::mesh::{Mesh, MeshBuilder, PrimitiveTopology};
// use bevy::prelude::{Measured3d, Primitive3d};
//
// pub struct HexagonalPrism {
//     size: f32,
//     height: f32,
// }
//
// impl Primitive3d for HexagonalPrism {}
//
// impl Default for HexagonalPrism {
//     fn default() -> Self {
//         Self {
//             size: 1.0,
//             height: 1.0,
//         }
//     }
// }
//
// impl HexagonalPrism {
//     pub const fn new(size: f32, height: f32) -> Self {
//         Self { size, height }
//     }
// }
//
// const SQRT_3: f32 = 1.732050807568877293527446341505872367_f32;
// const THREE_HALVES_SQRT_3: f32 = SQRT_3 * 3.0 / 2.0;
//
// impl Measured3d for HexagonalPrism {
//     fn area(&self) -> f32 {
//         3.0 * self.size * (SQRT_3 * self.size + 2.0 * self.height)
//     }
//
//     fn volume(&self) -> f32 {
//         THREE_HALVES_SQRT_3 * self.size.squared() * self.height
//     }
// }
//
// pub struct HexagonalPrismMeshBuilder(HexagonalPrism);
//
// impl Default for HexagonalPrismMeshBuilder {
//     fn default() -> Self {
//         HexagonalPrismMeshBuilder(HexagonalPrism::default())
//     }
// }
//
// // To get the normal for a side:
// // n = (p₁ - p₀) x (p₂ - p₀)
// // Where p₀₋₂ are points on the side
// // For sides not directly facing one of the four cardinal directions
// // and the points that are 1 unit away from each other:
// // that results in abs(x) = 2 and abs(z) = 1
// // We divide that by the magnitude of the normal vector: √(2² + 1²)
// // to normalize the vector
// // These two constants represent the normal vector for the right front side,
// // but by inverting the sign of x, z or both you can get the vectors for all
// // other angled sides
// const ANGLED_NORMAL_X: f32 = 0.8944272; // 2/SQRT(5)
// const ANGLED_NORMAL_Z: f32 = 0.4472136; // 2/SQRT(5)
//
// impl HexagonalPrismMeshBuilder {
//     fn dimensions(&self) -> (Vec<[f32; 3]>, Vec<Vec3>) {
//         //Vec3(0.8944272, 0.0, -0.4472136)
//         let half_height = self.0.height / 2.0;
//         let half_size = self.0.size / 2.0;
//
//         // Suppose Y-up right hand, and camera look from +Z to -Z
//         let vertices = &[
//             // Front (bottom-left, bottom right, top right, top left)
//             ([-half_size, -half_height, self.0.size], [0.0, 0.0, 1.0]),
//             ([half_size, -half_height, self.0.size], [0.0, 0.0, 1.0]),
//             ([half_size, half_height, self.0.size], [0.0, 0.0, 1.0]),
//             ([-half_size, half_height, self.0.size], [0.0, 0.0, 1.0]),
//             // Front-Right
//             (
//                 [half_size, -half_height, self.0.size],
//                 [ANGLED_NORMAL_X, 0.0, ANGLED_NORMAL_Z],
//             ),
//             (
//                 [self.0.size, -half_height, 0.0],
//                 [ANGLED_NORMAL_X, 0.0, ANGLED_NORMAL_Z],
//             ),
//             (
//                 [self.0.size, half_height, 0.0],
//                 [ANGLED_NORMAL_X, 0.0, ANGLED_NORMAL_Z],
//             ),
//             (
//                 [half_size, half_height, self.0.size],
//                 [ANGLED_NORMAL_X, 0.0, ANGLED_NORMAL_Z],
//             ),
//             // Back-Right
//             (
//                 [self.0.size, -half_height, 0.0],
//                 [ANGLED_NORMAL_X, 0.0, -ANGLED_NORMAL_Z],
//             ),
//             (
//                 [half_size, -half_height, -self.0.size],
//                 [ANGLED_NORMAL_X, 0.0, -ANGLED_NORMAL_Z],
//             ),
//             (
//                 [half_size, half_height, -self.0.size],
//                 [ANGLED_NORMAL_X, 0.0, -ANGLED_NORMAL_Z],
//             ),
//             (
//                 [self.0.size, half_height, 0.0],
//                 [ANGLED_NORMAL_X, 0.0, -ANGLED_NORMAL_Z],
//             ),
//             // Back
//             ([half_size, -half_height, -self.0.size], [0.0, 0.0, 1.0]),
//             ([-half_size, -half_height, -self.0.size], [0.0, 0.0, 1.0]),
//             ([-half_size, half_height, -self.0.size], [0.0, 0.0, 1.0]),
//             ([half_size, half_height, -self.0.size], [0.0, 0.0, 1.0]),
//             // Back-Left
//             (
//                 [-half_size, -half_height, -self.0.size],
//                 [-ANGLED_NORMAL_X, 0.0, -ANGLED_NORMAL_Z],
//             ),
//             (
//                 [-self.0.size, -half_height, 0.0],
//                 [-ANGLED_NORMAL_X, 0.0, -ANGLED_NORMAL_Z],
//             ),
//             (
//                 [-self.0.size, half_height, 0.0],
//                 [-ANGLED_NORMAL_X, 0.0, -ANGLED_NORMAL_Z],
//             ),
//             (
//                 [-half_size, half_height, -self.0.size],
//                 [-ANGLED_NORMAL_X, 0.0, -ANGLED_NORMAL_Z],
//             ),
//             // Front-Left
//             (
//                 [-self.0.size, -half_height, 0.0],
//                 [-ANGLED_NORMAL_X, 0.0, ANGLED_NORMAL_Z],
//             ),
//             (
//                 [-half_size, -half_height, self.0.size],
//                 [-ANGLED_NORMAL_X, 0.0, ANGLED_NORMAL_Z],
//             ),
//             (
//                 [-half_size, half_height, self.0.size],
//                 [-ANGLED_NORMAL_X, 0.0, ANGLED_NORMAL_Z],
//             ),
//             (
//                 [-self.0.size, half_height, 0.0],
//                 [-ANGLED_NORMAL_X, 0.0, ANGLED_NORMAL_Z],
//             ),
//             // Top (counter-clockwise starting front-left)
//             ([-half_size, half_height, self.0.size], [0.0, 1.0, 0.0]),
//             ([half_size, half_height, self.0.size], [0.0, 1.0, 0.0]),
//             ([self.0.size, half_height, 0.0], [0.0, 1.0, 0.0]),
//             ([half_size, half_height, -self.0.size], [0.0, 1.0, 0.0]),
//             ([-half_size, half_height, -self.0.size], [0.0, 1.0, 0.0]),
//             ([-self.0.size, half_height, 0.0], [0.0, 1.0, 0.0]),
//             // Bottom (counter-clockwise starting front-left)
//             ([-half_size, -half_height, self.0.size], [0.0, -1.0, 0.0]),
//             ([half_size, -half_height, self.0.size], [0.0, -1.0, 0.0]),
//             ([self.0.size, -half_height, 0.0], [0.0, -1.0, 0.0]),
//             ([half_size, -half_height, -self.0.size], [0.0, -1.0, 0.0]),
//             ([-half_size, -half_height, -self.0.size], [0.0, -1.0, 0.0]),
//             ([-self.0.size, -half_height, 0.0], [0.0, -1.0, 0.0]),
//         ];
//
//         // (Vec::from(vertices), Vec::from(normals))
//         todo!()
//     }
// }
// impl MeshBuilder for HexagonalPrismMeshBuilder {
//     fn build(&self) -> Mesh {
//         // let half_height = self.0.height / 2.0;
//
//         // let mut normals: [Vec3; 12] = [Vec3::ZERO; 12];
//         // for i in 0..12 {
//         //     let point_one = Vec3::from(vertices[i * 4]);
//         //     let point_two = Vec3::from(vertices[i * 4 + 1]);
//         //     let point_three = Vec3::from(vertices[i * 4 + 2]);
//         //     normals[i] = (point_two - point_one)
//         //         .cross(point_three - point_two)
//         //         .normalize();
//         // }
//
//         Mesh::new(
//             PrimitiveTopology::TriangleList,
//             RenderAssetUsages::default(),
//         )
//         .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vec![Vec3::new(1.0, 1.0, 1.0)])
//         .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![Vec3::new(1.0, 1.0, 1.0)])
//     }
// }
//
// #[cfg(test)]
// mod tests {
//     use super::*;
//
//     #[test]
//     fn test_build_mesh() {
//         let builder = HexagonalPrismMeshBuilder(HexagonalPrism {
//             size: 1.0,
//             height: 1.0,
//         });
//         assert_eq!((Vec::new(), Vec::new()), builder.dimensions());
//     }
// }
