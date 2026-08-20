//! Validated GNM Head v3 data and sparse evaluation.

use crate::error::GnmModelError;

/// Number of identity coefficients in the official GNM Head v3 model.
pub const GNM_HEAD_V3_IDENTITY_DIM: usize = 253;
/// Number of expression coefficients in the official GNM Head v3 model.
pub const GNM_HEAD_V3_EXPRESSION_DIM: usize = 383;

/// GNM model version.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GnmVersion {
    /// Major schema version.
    pub major: u16,
    /// Minor schema version.
    pub minor: u16,
}

/// Supported GNM model variant.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GnmVariant {
    /// The v3 head model used by the retargeting pipeline.
    Head,
}

/// A finite, row-major numeric tensor.
#[derive(Clone, Debug, PartialEq)]
pub struct DenseArray {
    shape: Vec<usize>,
    values: Vec<f32>,
}

impl DenseArray {
    /// Creates a tensor after checking its dimensions and finiteness.
    pub fn new(
        field: impl Into<String>,
        shape: Vec<usize>,
        values: Vec<f32>,
    ) -> Result<Self, GnmModelError> {
        let field = field.into();
        let expected = shape
            .iter()
            .try_fold(1usize, |product, dimension| product.checked_mul(*dimension));
        let Some(expected) = expected else {
            return Err(GnmModelError::InvalidValue {
                field,
                reason: "shape size overflow".to_owned(),
            });
        };
        if expected != values.len() {
            return Err(GnmModelError::Shape {
                field,
                expected: format!("{} values", expected),
                actual: format!("{} values", values.len()),
            });
        }
        if let Some(index) = values.iter().position(|value| !value.is_finite()) {
            return Err(GnmModelError::NonFinite { field, index });
        }
        Ok(Self { shape, values })
    }

    /// Returns the tensor shape.
    pub fn shape(&self) -> &[usize] {
        &self.shape
    }

    /// Returns the flattened tensor values.
    pub fn values(&self) -> &[f32] {
        &self.values
    }

    fn at3(&self, first: usize, second: usize, third: usize) -> f32 {
        let index = first * self.shape[1] * self.shape[2] + second * self.shape[2] + third;
        self.values[index]
    }
}

/// Identity coefficients for one GNM evaluation.
#[derive(Clone, Debug, PartialEq)]
pub struct GnmIdentityState(Vec<f32>);

impl GnmIdentityState {
    /// Creates an identity state with the requested model dimension.
    pub fn new(values: Vec<f32>, expected_dimension: usize) -> Result<Self, GnmModelError> {
        validate_state("identity", values.len(), expected_dimension)?;
        validate_finite("identity", &values)?;
        Ok(Self(values))
    }

    /// Creates the neutral identity state for a model.
    pub fn neutral(expected_dimension: usize) -> Self {
        Self(vec![0.0; expected_dimension])
    }

    /// Returns the coefficients.
    pub fn values(&self) -> &[f32] {
        &self.0
    }
}

/// Expression coefficients for one GNM evaluation.
#[derive(Clone, Debug, PartialEq)]
pub struct GnmExpressionState(Vec<f32>);

impl GnmExpressionState {
    /// Creates an expression state with the requested model dimension.
    pub fn new(values: Vec<f32>, expected_dimension: usize) -> Result<Self, GnmModelError> {
        validate_state("expression", values.len(), expected_dimension)?;
        validate_finite("expression", &values)?;
        Ok(Self(values))
    }

    /// Creates the neutral expression state for a model.
    pub fn neutral(expected_dimension: usize) -> Self {
        Self(vec![0.0; expected_dimension])
    }

    /// Returns the coefficients.
    pub fn values(&self) -> &[f32] {
        &self.0
    }
}

/// Optional joint pose state. Rotations are axis-angle vectors in radians.
#[derive(Clone, Debug, PartialEq)]
pub struct GnmJointState {
    rotations: Vec<[f32; 3]>,
    translation: [f32; 3],
}

impl GnmJointState {
    /// Creates a joint state with one rotation per model joint.
    pub fn new(
        rotations: Vec<[f32; 3]>,
        translation: [f32; 3],
        expected_joint_count: usize,
    ) -> Result<Self, GnmModelError> {
        if rotations.len() != expected_joint_count {
            return Err(GnmModelError::Shape {
                field: "joint_rotations".to_owned(),
                expected: format!("[{expected_joint_count}, 3]"),
                actual: format!("[{}, 3]", rotations.len()),
            });
        }
        if rotations.iter().flatten().any(|value| !value.is_finite())
            || translation.iter().any(|value| !value.is_finite())
        {
            return Err(GnmModelError::NonFinite {
                field: "joint_state".to_owned(),
                index: 0,
            });
        }
        Ok(Self {
            rotations,
            translation,
        })
    }

    /// Creates a neutral pose state for a model.
    pub fn neutral(expected_joint_count: usize) -> Self {
        Self {
            rotations: vec![[0.0; 3]; expected_joint_count],
            translation: [0.0; 3],
        }
    }

    /// Returns joint axis-angle rotations.
    pub fn rotations(&self) -> &[[f32; 3]] {
        &self.rotations
    }

    /// Returns the global translation.
    pub fn translation(&self) -> [f32; 3] {
        self.translation
    }
}

/// Input arrays needed by the sparse Head v3 evaluator.
#[derive(Clone, Debug, PartialEq)]
pub struct GnmModelData {
    /// Model schema version.
    pub version: GnmVersion,
    /// Model variant.
    pub variant: GnmVariant,
    /// Neutral vertices, shaped `[vertex, 3]`.
    pub template_vertices: DenseArray,
    /// Neutral joints, shaped `[joint, 3]`.
    pub template_joints: DenseArray,
    /// Identity vertex basis, shaped `[identity, vertex, 3]`.
    pub vertex_identity_basis: DenseArray,
    /// Identity joint basis, shaped `[identity, joint, 3]`.
    pub joint_identity_basis: DenseArray,
    /// Expression vertex basis, shaped `[expression, vertex, 3]`.
    pub expression_basis: DenseArray,
    /// Parent index for each joint; the root may use `-1` or the official
    /// self-parent sentinel `0`.
    pub joint_parent_indices: Vec<i32>,
    /// Linear blend skinning weights, shaped `[joint, vertex]`.
    pub skinning_weights: DenseArray,
    /// Optional pose-corrective basis, shaped `[9 * joint, vertex, 3]`.
    pub pose_correctives_regressor: Option<DenseArray>,
}

/// Validated GNM model.
#[derive(Clone, Debug, PartialEq)]
pub struct GnmModel {
    version: GnmVersion,
    variant: GnmVariant,
    template_vertices: DenseArray,
    template_joints: DenseArray,
    vertex_identity_basis: DenseArray,
    joint_identity_basis: DenseArray,
    expression_basis: DenseArray,
    joint_parent_indices: Vec<i32>,
    skinning_weights: DenseArray,
    pose_correctives_regressor: Option<DenseArray>,
}

impl GnmModel {
    /// Validates a Head v3 model and takes ownership of its numeric arrays.
    pub fn from_data(data: GnmModelData) -> Result<Self, GnmModelError> {
        if data.version.major != 3 {
            return Err(GnmModelError::UnsupportedVersion(format!(
                "{}.{}",
                data.version.major, data.version.minor
            )));
        }
        if data.variant != GnmVariant::Head {
            return Err(GnmModelError::UnsupportedVariant(format!(
                "{:?}",
                data.variant
            )));
        }
        let [vertex_count, three] = shape2("template_vertices", data.template_vertices.shape())?;
        if three != 3 {
            return Err(shape_error(
                "template_vertices",
                "[vertex, 3]",
                data.template_vertices.shape(),
            ));
        }
        let [joint_count, joint_three] = shape2("template_joints", data.template_joints.shape())?;
        if joint_three != 3 {
            return Err(shape_error(
                "template_joints",
                "[joint, 3]",
                data.template_joints.shape(),
            ));
        }
        let [identity_count, identity_vertices, identity_three] =
            shape3("vertex_identity_basis", data.vertex_identity_basis.shape())?;
        if [identity_vertices, identity_three] != [vertex_count, 3] {
            return Err(shape_error(
                "vertex_identity_basis",
                "[identity, vertex, 3]",
                data.vertex_identity_basis.shape(),
            ));
        }
        let [
            joint_identity_count,
            joint_identity_joints,
            joint_identity_three,
        ] = shape3("joint_identity_basis", data.joint_identity_basis.shape())?;
        if [
            joint_identity_count,
            joint_identity_joints,
            joint_identity_three,
        ] != [identity_count, joint_count, 3]
        {
            return Err(shape_error(
                "joint_identity_basis",
                "[identity, joint, 3] matching vertex identity basis",
                data.joint_identity_basis.shape(),
            ));
        }
        let [expression_count, expression_vertices, expression_three] =
            shape3("expression_basis", data.expression_basis.shape())?;
        if [expression_vertices, expression_three] != [vertex_count, 3] {
            return Err(shape_error(
                "expression_basis",
                "[expression, vertex, 3]",
                data.expression_basis.shape(),
            ));
        }
        if data.joint_parent_indices.len() != joint_count {
            return Err(GnmModelError::Shape {
                field: "joint_parent_indices".to_owned(),
                expected: format!("[{joint_count}]"),
                actual: format!("[{}]", data.joint_parent_indices.len()),
            });
        }
        if joint_count == 0 {
            return Err(GnmModelError::InvalidValue {
                field: "template_joint_positions".to_owned(),
                reason: "at least one root joint is required".to_owned(),
            });
        }
        let root_parent = data.joint_parent_indices[0];
        if !matches!(root_parent, -1 | 0)
            || data
                .joint_parent_indices
                .iter()
                .enumerate()
                .skip(1)
                .any(|(index, parent)| (*parent < 0) || (*parent >= index as i32))
        {
            return Err(GnmModelError::InvalidValue {
                field: "joint_parent_indices".to_owned(),
                reason: "root must be -1 or 0 and children must reference an earlier joint"
                    .to_owned(),
            });
        }
        let weights_shape = data.skinning_weights.shape();
        if weights_shape != [joint_count, vertex_count] {
            return Err(shape_error(
                "skinning_weights",
                "[joint, vertex]",
                weights_shape,
            ));
        }
        if data
            .skinning_weights
            .values()
            .iter()
            .any(|weight| *weight < 0.0)
        {
            return Err(GnmModelError::InvalidValue {
                field: "skinning_weights".to_owned(),
                reason: "weights must be non-negative".to_owned(),
            });
        }
        if let Some(correctives) = &data.pose_correctives_regressor {
            let expected = [joint_count * 9, vertex_count, 3];
            if correctives.shape() != expected {
                return Err(shape_error(
                    "pose_correctives_regressor",
                    "[9 * joint, vertex, 3]",
                    correctives.shape(),
                ));
            }
        }
        if identity_count != GNM_HEAD_V3_IDENTITY_DIM {
            return Err(GnmModelError::InvalidValue {
                field: "vertex_identity_basis".to_owned(),
                reason: format!(
                    "Head v3 requires {GNM_HEAD_V3_IDENTITY_DIM} identity coefficients"
                ),
            });
        }
        if expression_count != GNM_HEAD_V3_EXPRESSION_DIM {
            return Err(GnmModelError::InvalidValue {
                field: "expression_basis".to_owned(),
                reason: format!(
                    "Head v3 requires {GNM_HEAD_V3_EXPRESSION_DIM} expression coefficients"
                ),
            });
        }
        Ok(Self {
            version: data.version,
            variant: data.variant,
            template_vertices: data.template_vertices,
            template_joints: data.template_joints,
            vertex_identity_basis: data.vertex_identity_basis,
            joint_identity_basis: data.joint_identity_basis,
            expression_basis: data.expression_basis,
            joint_parent_indices: data.joint_parent_indices,
            skinning_weights: data.skinning_weights,
            pose_correctives_regressor: data.pose_correctives_regressor,
        })
    }

    /// Returns the schema version.
    pub fn version(&self) -> GnmVersion {
        self.version
    }

    /// Returns the model variant.
    pub fn variant(&self) -> GnmVariant {
        self.variant
    }

    /// Returns the vertex count.
    pub fn vertex_count(&self) -> usize {
        self.template_vertices.shape()[0]
    }

    /// Returns the joint count.
    pub fn joint_count(&self) -> usize {
        self.template_joints.shape()[0]
    }

    /// Returns the identity dimension.
    pub fn identity_dimension(&self) -> usize {
        self.vertex_identity_basis.shape()[0]
    }

    /// Returns the expression dimension.
    pub fn expression_dimension(&self) -> usize {
        self.expression_basis.shape()[0]
    }

    /// Creates a neutral identity state.
    pub fn neutral_identity(&self) -> GnmIdentityState {
        GnmIdentityState::neutral(self.identity_dimension())
    }

    /// Creates a neutral expression state.
    pub fn neutral_expression(&self) -> GnmExpressionState {
        GnmExpressionState::neutral(self.expression_dimension())
    }

    /// Evaluates only the requested sparse landmark points into reusable output.
    pub fn evaluate_sparse(
        &self,
        identity: &GnmIdentityState,
        expression: &GnmExpressionState,
        joints: &GnmJointState,
        landmarks: &crate::SparseLandmarkSet,
        output: &mut GnmSparseVertices,
    ) -> Result<(), GnmModelError> {
        validate_state(
            "identity",
            identity.values().len(),
            self.identity_dimension(),
        )?;
        validate_state(
            "expression",
            expression.values().len(),
            self.expression_dimension(),
        )?;
        if joints.rotations.len() != self.joint_count() {
            return Err(GnmModelError::Shape {
                field: "joint_rotations".to_owned(),
                expected: format!("[{}, 3]", self.joint_count()),
                actual: format!("[{}, 3]", joints.rotations.len()),
            });
        }
        output.resize(landmarks.len());
        let vertices = self.deformed_vertices(identity, expression, joints);
        let transforms = self.joint_transforms(joints, identity)?;
        for (point_index, (output_point, landmark)) in
            output.values.iter_mut().zip(landmarks.points()).enumerate()
        {
            let mut point: [f32; 3] = [0.0; 3];
            for (vertex, weight) in landmark.indices.iter().zip(landmark.weights) {
                if *vertex >= self.vertex_count() {
                    return Err(GnmModelError::InvalidValue {
                        field: "landmark vertex index".to_owned(),
                        reason: format!("point {point_index} references vertex {vertex}"),
                    });
                }
                let mut skinned = [0.0; 3];
                for (joint, transform) in transforms.iter().enumerate() {
                    let skin_weight =
                        self.skinning_weights.values()[joint * self.vertex_count() + *vertex];
                    let rest_joint = self.template_joints.values()[joint * 3..joint * 3 + 3]
                        .try_into()
                        .map_err(|_| GnmModelError::InvalidValue {
                            field: "template_joint_positions".to_owned(),
                            reason: "joint position must contain three components".to_owned(),
                        })?;
                    let transformed = transform.apply_skin(
                        [
                            vertex_value(&vertices, *vertex, 0),
                            vertex_value(&vertices, *vertex, 1),
                            vertex_value(&vertices, *vertex, 2),
                        ],
                        rest_joint,
                    );
                    for component in 0..3 {
                        skinned[component] += skin_weight * transformed[component];
                    }
                }
                for component in 0..3 {
                    point[component] += weight * skinned[component];
                }
            }
            if point.iter().any(|value| !value.is_finite()) {
                return Err(GnmModelError::NonFinite {
                    field: "sparse_vertices".to_owned(),
                    index: point_index,
                });
            }
            *output_point = point;
        }
        Ok(())
    }

    fn deformed_vertices(
        &self,
        identity: &GnmIdentityState,
        expression: &GnmExpressionState,
        joints: &GnmJointState,
    ) -> Vec<f32> {
        let mut vertices = self.template_vertices.values().to_vec();
        let vertex_count = self.vertex_count();
        for (basis, coefficient) in identity.values().iter().enumerate() {
            for vertex in 0..vertex_count {
                for component in 0..3 {
                    vertices[vertex * 3 + component] +=
                        coefficient * self.vertex_identity_basis.at3(basis, vertex, component);
                }
            }
        }
        for (basis, coefficient) in expression.values().iter().enumerate() {
            for vertex in 0..vertex_count {
                for component in 0..3 {
                    vertices[vertex * 3 + component] +=
                        coefficient * self.expression_basis.at3(basis, vertex, component);
                }
            }
        }
        if let Some(correctives) = &self.pose_correctives_regressor {
            for joint in 0..self.joint_count() {
                let rotation = rotation_from_axis_angle(joints.rotations[joint]);
                for (row, rotation_row) in rotation.iter().enumerate() {
                    for (column, rotation_value) in rotation_row.iter().enumerate() {
                        let coefficient = *rotation_value - if row == column { 1.0 } else { 0.0 };
                        let basis = joint * 9 + row * 3 + column;
                        for vertex in 0..vertex_count {
                            for component in 0..3 {
                                vertices[vertex * 3 + component] +=
                                    coefficient * correctives.at3(basis, vertex, component);
                            }
                        }
                    }
                }
            }
        }
        vertices
    }

    fn joint_transforms(
        &self,
        joints: &GnmJointState,
        identity: &GnmIdentityState,
    ) -> Result<Vec<JointTransform>, GnmModelError> {
        let joint_count = self.joint_count();
        let mut local = vec![JointTransform::identity(); joint_count];
        for (joint, local_transform) in local.iter_mut().enumerate() {
            let mut position = [0.0; 3];
            for (component, position_component) in position.iter_mut().enumerate() {
                *position_component = self.template_joints.values()[joint * 3 + component];
                for (basis, coefficient) in identity.values().iter().enumerate() {
                    *position_component +=
                        coefficient * self.joint_identity_basis.at3(basis, joint, component);
                }
            }
            let parent = self.joint_parent_indices[joint];
            let local_position = if joint == 0 && (parent == -1 || parent == 0) {
                add(position, joints.translation)
            } else {
                let parent = parent as usize;
                let parent_position = [
                    self.template_joints.values()[parent * 3],
                    self.template_joints.values()[parent * 3 + 1],
                    self.template_joints.values()[parent * 3 + 2],
                ];
                [
                    position[0] - parent_position[0],
                    position[1] - parent_position[1],
                    position[2] - parent_position[2],
                ]
            };
            *local_transform = JointTransform {
                rotation: rotation_from_axis_angle(joints.rotations[joint]),
                translation: local_position,
            };
        }
        let mut world: Vec<JointTransform> = Vec::with_capacity(joint_count);
        for (joint, local_transform) in local.iter().enumerate() {
            let parent = self.joint_parent_indices[joint];
            if joint == 0 && (parent == -1 || parent == 0) {
                world.push(*local_transform);
            } else {
                let parent = parent as usize;
                if parent >= world.len() {
                    return Err(GnmModelError::InvalidValue {
                        field: "joint_parent_indices".to_owned(),
                        reason: "parents must precede their children".to_owned(),
                    });
                }
                world.push(world[parent].compose(*local_transform));
            }
        }
        Ok(world)
    }
}

fn validate_state(field: &str, actual: usize, expected: usize) -> Result<(), GnmModelError> {
    if actual != expected {
        return Err(GnmModelError::Shape {
            field: field.to_owned(),
            expected: format!("[{expected}]"),
            actual: format!("[{actual}]"),
        });
    }
    Ok(())
}

fn validate_finite(field: &str, values: &[f32]) -> Result<(), GnmModelError> {
    if let Some(index) = values.iter().position(|value| !value.is_finite()) {
        return Err(GnmModelError::NonFinite {
            field: field.to_owned(),
            index,
        });
    }
    Ok(())
}

fn shape2(field: &str, shape: &[usize]) -> Result<[usize; 2], GnmModelError> {
    shape
        .try_into()
        .map_err(|_| shape_error(field, "rank 2", shape))
}

fn shape3(field: &str, shape: &[usize]) -> Result<[usize; 3], GnmModelError> {
    shape
        .try_into()
        .map_err(|_| shape_error(field, "rank 3", shape))
}

fn shape_error(field: &str, expected: &str, actual: &[usize]) -> GnmModelError {
    GnmModelError::Shape {
        field: field.to_owned(),
        expected: expected.to_owned(),
        actual: format!("{actual:?}"),
    }
}

fn vertex_value(vertices: &[f32], vertex: usize, component: usize) -> f32 {
    vertices[vertex * 3 + component]
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct JointTransform {
    rotation: [[f32; 3]; 3],
    translation: [f32; 3],
}

impl JointTransform {
    const fn identity() -> Self {
        Self {
            rotation: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            translation: [0.0; 3],
        }
    }

    fn apply(self, point: [f32; 3]) -> [f32; 3] {
        add(matrix_vector(self.rotation, point), self.translation)
    }

    fn apply_skin(self, point: [f32; 3], rest_joint: [f32; 3]) -> [f32; 3] {
        self.apply([
            point[0] - rest_joint[0],
            point[1] - rest_joint[1],
            point[2] - rest_joint[2],
        ])
    }

    fn compose(self, child: Self) -> Self {
        Self {
            rotation: matrix_matrix(self.rotation, child.rotation),
            translation: add(
                matrix_vector(self.rotation, child.translation),
                self.translation,
            ),
        }
    }
}

/// Reusable sparse landmark output buffer.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GnmSparseVertices {
    values: Vec<[f32; 3]>,
}

impl GnmSparseVertices {
    /// Creates an output buffer for a fixed number of points.
    pub fn with_len(length: usize) -> Self {
        Self {
            values: vec![[0.0; 3]; length],
        }
    }

    fn resize(&mut self, length: usize) {
        self.values.resize(length, [0.0; 3]);
    }

    /// Returns the evaluated points.
    pub fn values(&self) -> &[[f32; 3]] {
        &self.values
    }

    /// Returns mutable evaluated points for a caller-owned reusable buffer.
    pub fn values_mut(&mut self) -> &mut [[f32; 3]] {
        &mut self.values
    }
}

fn add(left: [f32; 3], right: [f32; 3]) -> [f32; 3] {
    [left[0] + right[0], left[1] + right[1], left[2] + right[2]]
}

fn matrix_vector(matrix: [[f32; 3]; 3], vector: [f32; 3]) -> [f32; 3] {
    [
        matrix[0][0] * vector[0] + matrix[0][1] * vector[1] + matrix[0][2] * vector[2],
        matrix[1][0] * vector[0] + matrix[1][1] * vector[1] + matrix[1][2] * vector[2],
        matrix[2][0] * vector[0] + matrix[2][1] * vector[1] + matrix[2][2] * vector[2],
    ]
}

fn matrix_matrix(left: [[f32; 3]; 3], right: [[f32; 3]; 3]) -> [[f32; 3]; 3] {
    let mut result = [[0.0; 3]; 3];
    for row in 0..3 {
        for column in 0..3 {
            for index in 0..3 {
                result[row][column] += left[row][index] * right[index][column];
            }
        }
    }
    result
}

fn rotation_from_axis_angle(axis_angle: [f32; 3]) -> [[f32; 3]; 3] {
    let theta = (axis_angle[0] * axis_angle[0]
        + axis_angle[1] * axis_angle[1]
        + axis_angle[2] * axis_angle[2])
        .sqrt();
    if theta <= f32::EPSILON {
        return JointTransform::identity().rotation;
    }
    let axis = [
        axis_angle[0] / theta,
        axis_angle[1] / theta,
        axis_angle[2] / theta,
    ];
    let (sin, cos) = theta.sin_cos();
    let mut result = [[0.0; 3]; 3];
    for row in 0..3 {
        for column in 0..3 {
            let delta = if row == column { 1.0 } else { 0.0 };
            result[row][column] = cos * delta
                + (1.0 - cos) * axis[row] * axis[column]
                + sin * skew(axis)[row][column];
        }
    }
    result
}

fn skew(axis: [f32; 3]) -> [[f32; 3]; 3] {
    [
        [0.0, -axis[2], axis[1]],
        [axis[2], 0.0, -axis[0]],
        [-axis[1], axis[0], 0.0],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic_model() -> GnmModel {
        let identity = GNM_HEAD_V3_IDENTITY_DIM;
        let expression = GNM_HEAD_V3_EXPRESSION_DIM;
        GnmModel::from_data(GnmModelData {
            version: GnmVersion { major: 3, minor: 0 },
            variant: GnmVariant::Head,
            template_vertices: DenseArray::new(
                "vertices",
                vec![3, 3],
                vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0],
            )
            .unwrap(),
            template_joints: DenseArray::new("joints", vec![1, 3], vec![0.0, 0.0, 0.0]).unwrap(),
            vertex_identity_basis: DenseArray::new(
                "identity",
                vec![identity, 3, 3],
                vec![0.0; identity * 3 * 3],
            )
            .unwrap(),
            joint_identity_basis: DenseArray::new(
                "joint_identity",
                vec![identity, 1, 3],
                vec![0.0; identity * 3],
            )
            .unwrap(),
            expression_basis: DenseArray::new(
                "expression",
                vec![expression, 3, 3],
                vec![0.0; expression * 3 * 3],
            )
            .unwrap(),
            joint_parent_indices: vec![-1],
            skinning_weights: DenseArray::new("weights", vec![1, 3], vec![1.0; 3]).unwrap(),
            pose_correctives_regressor: None,
        })
        .unwrap()
    }

    #[test]
    fn sparse_evaluation_is_barycentric_and_reusable() {
        let model = synthetic_model();
        let landmarks = crate::SparseLandmarkSet::new(vec![
            crate::SparseLandmark::new([0, 1, 2], [0.25, 0.25, 0.5]).unwrap(),
        ])
        .unwrap();
        let identity = model.neutral_identity();
        let expression = model.neutral_expression();
        let joints = GnmJointState::neutral(1);
        let mut output = GnmSparseVertices::with_len(1);
        let capacity = output.values.capacity();
        model
            .evaluate_sparse(&identity, &expression, &joints, &landmarks, &mut output)
            .unwrap();
        assert_eq!(output.values()[0], [0.25, 0.5, 0.0]);
        model
            .evaluate_sparse(&identity, &expression, &joints, &landmarks, &mut output)
            .unwrap();
        assert_eq!(output.values.capacity(), capacity);
    }

    #[test]
    fn wrong_head_dimensions_are_rejected() {
        let model = synthetic_model();
        let data = GnmModelData {
            version: model.version,
            variant: model.variant,
            template_vertices: model.template_vertices,
            template_joints: model.template_joints,
            vertex_identity_basis: DenseArray::new("identity", vec![1, 3, 3], vec![0.0; 9])
                .unwrap(),
            joint_identity_basis: model.joint_identity_basis,
            expression_basis: model.expression_basis,
            joint_parent_indices: model.joint_parent_indices,
            skinning_weights: model.skinning_weights,
            pose_correctives_regressor: model.pose_correctives_regressor,
        };
        assert!(
            matches!(GnmModel::from_data(data), Err(GnmModelError::Shape { field, .. }) if field == "joint_identity_basis")
        );
    }
}
