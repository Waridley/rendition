use std::cmp::Ordering;
use avian3d::parry::bounding_volume::{Aabb, BoundingSphere};
use avian3d::parry::mass_properties::MassProperties;
use avian3d::parry::math::Vector;
use avian3d::parry::na::{distance, UnitQuaternion};
use avian3d::parry::query::{
	ClosestPoints, Contact, NonlinearRigidMotion, PointProjection, PointQuery, Ray, RayCast,
	RayIntersection, ShapeCastHit, ShapeCastOptions, Unsupported,
};
use avian3d::parry::shape::{
	ConvexPolyhedron, FeatureId, PackedFeatureId, PolygonalFeature, PolygonalFeatureMap,
	RoundConvexPolyhedron, ShapeType, SupportMap,
};
use avian3d::parry::{
	math::Isometry,
	math::Point,
	na::Unit,
	na::{Isometry2, Vector3},
	query::{DefaultQueryDispatcher, QueryDispatcher},
	shape::HalfSpace,
	shape::{Shape, SharedShape, TypedShape},
};
use bevy::prelude::*;
use num_traits::identities::Zero;
use parry2d::query::PointQuery as PointQuery2d;
use parry2d::shape::SharedShape as Shape2d;
use parry2d::{na::Vector2, shape::Cuboid as Cuboid2d};

/// Projects a 3D shape onto the XZ plane (Z-up, Y-forward) and returns the projected 2D shape,
/// its 2D position within the plane, and the 3D distance of `pos.translation` from the plane.
///
/// In our right-handed Z-up, Y-forward convention, the plane normal is +Y (forward), and the plane
/// itself is the XZ ground plane transformed by `origin`.
fn project_shape_onto_plane(
	origin: &Isometry<f32>,
	shape: &dyn Shape,
	pos: &Isometry<f32>,
) -> (Shape2d, Isometry2<f32>, f32) {
	let local = origin.inv_mul(pos);
	let dist = local.translation.vector.y;
	let local_2d = Isometry2::new(
		local.translation.vector.xz(),
		local.rotation.euler_angles().2,
	);

	// Transform a local point relative to `pos` to a local point relative to `local_2d`.
	let local_point_2d = |p: &Point<f32>| {
		local_2d
			.inverse_transform_point(&origin.inverse_transform_point(&pos.transform_point(p)).xz())
	};

	let shape_2d = match shape.as_typed_shape() {
		TypedShape::Ball(ball) => Shape2d::ball(ball.radius),
		TypedShape::Cuboid(cuboid) => {
			todo!("Cuboid probably needs projected to a polygon");
		}
		TypedShape::Capsule(capsule) => {
			let a = local_point_2d(&capsule.segment.a);
			let b = local_point_2d(&capsule.segment.b);
			Shape2d::capsule(a, b, capsule.radius)
		}
		TypedShape::Segment(segment) => {
			let a = local_point_2d(&segment.a);
			let b = local_point_2d(&segment.b);
			Shape2d::segment(a, b)
		}
		TypedShape::Triangle(triangle) => {
			let a = local_point_2d(&triangle.a);
			let b = local_point_2d(&triangle.b);
			let c = local_point_2d(&triangle.c);
			Shape2d::triangle(a, b, c)
		}
		TypedShape::TriMesh(trimesh) => {
			let verts = trimesh.vertices().iter().map(local_point_2d).collect();
			Shape2d::trimesh(verts, trimesh.indices().to_vec())
		}
		TypedShape::Polyline(polyline) => {
			let verts = polyline.vertices().iter().map(local_point_2d).collect();
			Shape2d::polyline(verts, Some(polyline.indices().to_vec()))
		}
		TypedShape::HalfSpace(halfspace) => {
			// Note that this is a projection, not intersection. Probably not useful that often, but
			// technically the correct implementation.
			let local_norm = origin.rotation * halfspace.normal;
			let n = Unit::new_normalize(local_norm.xz());
			Shape2d::halfspace(n)
		}
		TypedShape::HeightField(_) => {
			todo!("Should all points be projected? Do we even need HeightField projections?")
		}
		TypedShape::Compound(compound) => Shape2d::compound(
			compound
				.shapes()
				.iter()
				.map(|(pos, shape)| {
					let (shape, pos, _) = project_shape_onto_plane(origin, &**shape, pos);
					(pos, shape)
				})
				.collect(),
		),
		TypedShape::ConvexPolyhedron(_) => {
			todo!("ConvexPolyhedron is definitely possible but not done yet")
		}
		TypedShape::Cylinder(_) => todo!("Cylinder end caps need projected as arcs somehow"),
		TypedShape::Cone(_) => todo!("Cone end cap needs projected as arc somehow"),
		TypedShape::RoundCuboid(_) => todo!("RoundCuboid needs projected as rounded polygon"),
		TypedShape::RoundTriangle(round_triangle) => {
			let a = local_point_2d(&round_triangle.inner_shape.a);
			let b = local_point_2d(&round_triangle.inner_shape.b);
			let c = local_point_2d(&round_triangle.inner_shape.c);
			Shape2d::round_triangle(a, b, c, round_triangle.border_radius)
		}
		TypedShape::RoundCylinder(_) => {
			todo!("RoundCylinder end caps need projected as arcs somehow")
		}
		TypedShape::RoundCone(_) => todo!("RoundCone end cap needs projected as arc somehow"),
		TypedShape::RoundConvexPolyhedron(_) => {
			todo!("RoundConvexPolyhedron is definitely possible but not done yet")
		}
		TypedShape::Custom(_) => todo!("Custom shapes are not supported"),
	};

	(shape_2d, local_2d, dist)
}

/// A 2D rectangle embedded in the 3D XZ plane (Z-up, Y-forward).
///
/// - 2D X axis aligns with 3D X.
/// - 2D Y axis aligns with 3D Z (up).
/// - The plane’s normal is +Y (forward).
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Rect3d {
	pub rect: Cuboid2d,
}

impl Rect3d {
	pub fn new(half_extents: Vector2<f32>) -> Self {
		Self {
			rect: Cuboid2d::new(half_extents),
		}
	}
}

impl From<Cuboid2d> for Rect3d {
	fn from(rect: Cuboid2d) -> Self {
		Self { rect }
	}
}

impl RayCast for Rect3d {
	fn cast_local_ray_and_get_normal(
		&self,
		ray: &Ray,
		max_time_of_impact: f32,
		solid: bool,
	) -> Option<RayIntersection> {
		// If the ray is pointing away from the plane, there is no intersection.
		if ray.origin.y.signum() == ray.dir.y.signum() {
			return None;
		}
		let time_of_impact = -ray.origin.y / ray.dir.y;
		if time_of_impact > max_time_of_impact {
			return None;
		}
		let pt = ray.origin + (ray.dir * time_of_impact);
		if !self.rect.contains_local_point(&pt.xz()) {
			return None;
		}
		let normal = match ray.dir.y.partial_cmp(&0.0) {
			Some(Ordering::Less) => -*Vector3::y_axis(),
			Some(Ordering::Greater) => *Vector3::y_axis(),
			_ => {
				if ray.origin.y == 0.0 {
					// Ray is within the 2D plane
					let hit = parry2d::query::RayCast::cast_local_ray_and_get_normal(
						&self.rect,
						&parry2d::query::Ray { origin: pt.xz(), dir: ray.dir.xz() },
						max_time_of_impact,
						true
					)?;
					return Some(RayIntersection {
						time_of_impact: hit.time_of_impact,
						normal: Vector3::new(hit.normal.x, 0.0, hit.normal.y),
						feature: match hit.feature {
							parry2d::shape::FeatureId::Face(i) => FeatureId::Edge(i),
							parry2d::shape::FeatureId::Vertex(i) => FeatureId::Vertex(i),
							parry2d::shape::FeatureId::Unknown => FeatureId::Unknown,
						},
					})
				} else {
					// Ray is parallel to plane and not coplanar with it.
					return None;
				}
			},
		};
		Some(RayIntersection {
			time_of_impact,
			normal,
			feature: FeatureId::Face(0),
		})
	}
}

impl PointQuery for Rect3d {
	#[inline]
	fn project_local_point(&self, pt: &Point<f32>, _solid: bool) -> PointProjection {
		let proj = self.rect.project_local_point(&pt.xz(), true);
		PointProjection {
			is_inside: pt.y == 0.0 && proj.is_inside,
			point: Point::new(proj.point.x, 0.0, proj.point.y),
		}
	}

	#[inline]
	fn project_local_point_and_get_feature(&self, pt: &Point<f32>) -> (PointProjection, FeatureId) {
		(self.project_local_point(pt, false), FeatureId::Face(0))
	}
}

impl Shape for Rect3d {
	fn compute_local_aabb(&self) -> Aabb {
		Aabb::new(
			Point::new(-self.rect.half_extents.x, 0.0, -self.rect.half_extents.y),
			Point::new(self.rect.half_extents.x, 0.0, self.rect.half_extents.y),
		)
	}

	fn compute_local_bounding_sphere(&self) -> BoundingSphere {
		BoundingSphere::new(Point::origin(), self.rect.half_extents.norm())
	}

	fn clone_dyn(&self) -> Box<dyn Shape> {
		Box::new(self.clone())
	}

	fn scale_dyn(&self, scale: &Vector<f32>, _num_subdivisions: u32) -> Option<Box<dyn Shape>> {
		Some(Box::new(Self {
			rect: self.rect.scaled(&scale.xz()),
		}))
	}

	fn mass_properties(&self, density: f32) -> MassProperties {
		MassProperties::zero()
	}

	fn is_convex(&self) -> bool {
		// Can be treated as convex for collision detection purposes, just like triangle
		true
	}

	fn shape_type(&self) -> ShapeType {
		ShapeType::Custom
	}

	fn as_typed_shape(&'_ self) -> TypedShape<'_> {
		TypedShape::Custom(self)
	}

	fn ccd_thickness(&self) -> f32 {
		// same as triangle
		0.0
	}

	fn ccd_angular_thickness(&self) -> f32 {
		// same as triangle
		std::f32::consts::FRAC_PI_2
	}

	fn as_support_map(&self) -> Option<&dyn SupportMap> {
		Some(self)
	}

	fn as_polygonal_feature_map(&self) -> Option<(&dyn PolygonalFeatureMap, f32)> {
		Some((self, 0.0))
	}

	fn feature_normal_at_point(
		&self,
		feature: FeatureId,
		_point: &Point<f32>,
	) -> Option<Unit<Vector<f32>>> {
		const SQRT_2: f32 = std::f32::consts::FRAC_1_SQRT_2;
		match feature {
			FeatureId::Vertex(0) => Some(Unit::new_unchecked(Vector3::new(SQRT_2, 0.0, SQRT_2))),
			FeatureId::Vertex(1) => Some(Unit::new_unchecked(Vector3::new(-SQRT_2, 0.0, SQRT_2))),
			FeatureId::Vertex(2) => Some(Unit::new_unchecked(Vector3::new(-SQRT_2, 0.0, -SQRT_2))),
			FeatureId::Vertex(3) => Some(Unit::new_unchecked(Vector3::new(SQRT_2, 0.0, -SQRT_2))),
			FeatureId::Edge(0) => Some(Vector3::z_axis()),
			FeatureId::Edge(1) => Some(-Vector3::x_axis()),
			FeatureId::Edge(2) => Some(-Vector3::z_axis()),
			FeatureId::Edge(3) => Some(Vector3::x_axis()),
			FeatureId::Face(0) => Some(Vector3::y_axis()),
			_ => None,
		}
	}
}

impl SupportMap for Rect3d {
	fn local_support_point(&self, dir: &Vector3<f32>) -> Point<f32> {
		let pt = parry2d::shape::SupportMap::local_support_point(&self.rect, &dir.xz());
		Point::new(pt.x, 0.0, pt.y)
	}
}

impl PolygonalFeatureMap for Rect3d {
	fn local_support_feature(&self, dir: &Unit<Vector3<f32>>, out_feature: &mut PolygonalFeature) {
		*out_feature = PolygonalFeature {
			vertices: [
				// counter-clockwise, like increasing angle from 0.0
				Point::new(self.rect.half_extents.x, 0.0, self.rect.half_extents.y),
				Point::new(-self.rect.half_extents.x, 0.0, self.rect.half_extents.y),
				Point::new(-self.rect.half_extents.x, 0.0, -self.rect.half_extents.y),
				Point::new(self.rect.half_extents.x, 0.0, -self.rect.half_extents.y),
			],
			vids: [
				PackedFeatureId::vertex(0),
				PackedFeatureId::vertex(1),
				PackedFeatureId::vertex(2),
				PackedFeatureId::vertex(3),
			],
			eids: [
				PackedFeatureId::edge(0),
				PackedFeatureId::edge(1),
				PackedFeatureId::edge(2),
				PackedFeatureId::edge(3),
			],
			fid: PackedFeatureId::face(0),
			num_vertices: 4,
		}
	}
}

impl Rect3d {
	pub fn intersection_test(
		&self,
		pos12: &Isometry<f32>,
		other: TypedShape,
	) -> std::result::Result<bool, Unsupported> {
		match other {
			TypedShape::Custom(other) => {
				if let Some(other) = other.downcast_ref::<Rect3d>() {
					todo!()
				} else {
					Err(Unsupported)
				}
			}
			other => {
				self.distance(pos12, other)
					.map(|dist| dist <= 0.0)
			}
			// // TODO: optimize intersection tests instead of using distance
			// TypedShape::Ball(_) => todo!(),
			// TypedShape::Cuboid(_) => todo!(),
			// TypedShape::Capsule(_) => todo!(),
			// TypedShape::Segment(_) => todo!(),
			// TypedShape::Triangle(_) => todo!(),
			// TypedShape::TriMesh(_) => todo!(),
			// TypedShape::Polyline(_) => todo!(),
			// TypedShape::HalfSpace(_) => todo!(),
			// TypedShape::HeightField(_) => todo!(),
			// TypedShape::Compound(_) => todo!(),
			// TypedShape::ConvexPolyhedron(_) => todo!(),
			// TypedShape::Cylinder(_) => todo!(),
			// TypedShape::Cone(_) => todo!(),
			// TypedShape::RoundCuboid(_) => todo!(),
			// TypedShape::RoundTriangle(_) => todo!(),
			// TypedShape::RoundCylinder(_) => todo!(),
			// TypedShape::RoundCone(_) => todo!(),
			// TypedShape::RoundConvexPolyhedron(_) => todo!(),
		}
	}

	pub fn distance(
		&self,
		pos12: &Isometry<f32>,
		other: TypedShape,
	) -> std::result::Result<f32, Unsupported> {
		match other {
			TypedShape::Custom(other) => {
				if let Some(other) = other.downcast_ref::<Rect3d>() {
					todo!()
				} else {
					Err(Unsupported)
				}
			}
			TypedShape::Ball(ball) => {
				let pt1 = self.project_local_point(&pos12.translation.vector.into(), true).point;
				let v2pt1 = pt1 - Point::from(pos12.translation.vector);
				let dist = v2pt1.norm() - ball.radius;
				Ok(f32::max(0.0, dist))
			},
			TypedShape::Cuboid(_) => todo!(),
			TypedShape::Capsule(_) => todo!(),
			TypedShape::Segment(_) => todo!(),
			TypedShape::Triangle(_) => todo!(),
			TypedShape::TriMesh(_) => todo!(),
			TypedShape::Polyline(_) => todo!(),
			TypedShape::HalfSpace(_) => todo!(),
			TypedShape::HeightField(_) => todo!(),
			TypedShape::Compound(_) => todo!(),
			TypedShape::ConvexPolyhedron(_) => todo!(),
			TypedShape::Cylinder(_) => todo!(),
			TypedShape::Cone(_) => todo!(),
			TypedShape::RoundCuboid(_) => todo!(),
			TypedShape::RoundTriangle(_) => todo!(),
			TypedShape::RoundCylinder(_) => todo!(),
			TypedShape::RoundCone(_) => todo!(),
			TypedShape::RoundConvexPolyhedron(_) => todo!(),
		}
	}

	pub fn contact(
		&self,
		pos12: &Isometry<f32>,
		other: TypedShape,
		prediction: f32,
	) -> std::result::Result<Option<Contact>, Unsupported> {
		match other {
			TypedShape::Custom(other) => {
				if let Some(other) = other.downcast_ref::<Rect3d>() {
					todo!()
				} else {
					Err(Unsupported)
				}
			}
			TypedShape::Ball(ball) => {
				let point1 = self.project_local_point(&pos12.translation.vector.into(), true).point;
				let v_ball_center_to_pt1 = point1 - Point::from(pos12.translation.vector);
				let dist = v_ball_center_to_pt1.norm() - ball.radius;
				let normal2 = Unit::new_normalize(pos12.inverse_transform_vector(&v_ball_center_to_pt1));
				let normal1 = -normal2;
				let point2 = (*normal2 * ball.radius).into();
				if dist > prediction {
					Ok(None)
				} else {
					Ok(Some(Contact {
						point1,
						point2,
						normal1,
						normal2,
						dist,
					}))
				}
			},
			TypedShape::Cuboid(_) => todo!(),
			TypedShape::Capsule(_) => todo!(),
			TypedShape::Segment(_) => todo!(),
			TypedShape::Triangle(_) => todo!(),
			TypedShape::TriMesh(_) => todo!(),
			TypedShape::Polyline(_) => todo!(),
			TypedShape::HalfSpace(_) => todo!(),
			TypedShape::HeightField(_) => todo!(),
			TypedShape::Compound(_) => todo!(),
			TypedShape::ConvexPolyhedron(_) => todo!(),
			TypedShape::Cylinder(_) => todo!(),
			TypedShape::Cone(_) => todo!(),
			TypedShape::RoundCuboid(_) => todo!(),
			TypedShape::RoundTriangle(_) => todo!(),
			TypedShape::RoundCylinder(_) => todo!(),
			TypedShape::RoundCone(_) => todo!(),
			TypedShape::RoundConvexPolyhedron(_) => todo!(),
		}
	}

	pub fn closest_points(
		&self,
		pos12: &Isometry<f32>,
		other: TypedShape,
		max_dist: f32,
	) -> std::result::Result<ClosestPoints, Unsupported> {
		match other {
			TypedShape::Custom(other) => {
				if let Some(other) = other.downcast_ref::<Rect3d>() {
					todo!()
				} else {
					Err(Unsupported)
				}
			}
			TypedShape::Ball(ball) => {
				if ball.radius >= pos12.translation.vector.y.abs() {
					Ok(ClosestPoints::Intersecting)
				} else {
					let pt1 = self.project_local_point(&pos12.translation.vector.into(), true).point;
					let v2pt1 = pt1 - Point::from(pos12.translation.vector);
					let dist = v2pt1.norm() - ball.radius;
					if dist > max_dist {
						Ok(ClosestPoints::Disjoint)
					} else {
						let pt2 = pos12.inverse_transform_vector(&v2pt1.normalize()) * ball.radius;
						Ok(ClosestPoints::WithinMargin(pt1, pt2.into()))
					}
				}
			},
			TypedShape::Cuboid(_) => todo!(),
			TypedShape::Capsule(_) => todo!(),
			TypedShape::Segment(_) => todo!(),
			TypedShape::Triangle(_) => todo!(),
			TypedShape::TriMesh(_) => todo!(),
			TypedShape::Polyline(_) => todo!(),
			TypedShape::HalfSpace(_) => todo!(),
			TypedShape::HeightField(_) => todo!(),
			TypedShape::Compound(_) => todo!(),
			TypedShape::ConvexPolyhedron(_) => todo!(),
			TypedShape::Cylinder(_) => todo!(),
			TypedShape::Cone(_) => todo!(),
			TypedShape::RoundCuboid(_) => todo!(),
			TypedShape::RoundTriangle(_) => todo!(),
			TypedShape::RoundCylinder(_) => todo!(),
			TypedShape::RoundCone(_) => todo!(),
			TypedShape::RoundConvexPolyhedron(_) => todo!(),
		}
	}

	pub fn cast_shapes(
		&self,
		pos12: &Isometry<f32>,
		local_vel12: &Vector<f32>,
		other: TypedShape,
		options: ShapeCastOptions,
	) -> std::result::Result<Option<ShapeCastHit>, Unsupported> {
		match other {
			TypedShape::Custom(other) => {
				if let Some(other) = other.downcast_ref::<Rect3d>() {
					todo!()
				} else {
					Err(Unsupported)
				}
			}
			TypedShape::Ball(ball) => {
				todo!()
			},
			TypedShape::Cuboid(_) => todo!(),
			TypedShape::Capsule(_) => todo!(),
			TypedShape::Segment(_) => todo!(),
			TypedShape::Triangle(_) => todo!(),
			TypedShape::TriMesh(_) => todo!(),
			TypedShape::Polyline(_) => todo!(),
			TypedShape::HalfSpace(_) => todo!(),
			TypedShape::HeightField(_) => todo!(),
			TypedShape::Compound(_) => todo!(),
			TypedShape::ConvexPolyhedron(_) => todo!(),
			TypedShape::Cylinder(_) => todo!(),
			TypedShape::Cone(_) => todo!(),
			TypedShape::RoundCuboid(_) => todo!(),
			TypedShape::RoundTriangle(_) => todo!(),
			TypedShape::RoundCylinder(_) => todo!(),
			TypedShape::RoundCone(_) => todo!(),
			TypedShape::RoundConvexPolyhedron(_) => todo!(),
		}
	}

	pub fn cast_shapes_nonlinear(
		&self,
		motion1: &NonlinearRigidMotion,
		other: TypedShape,
		motion2: &NonlinearRigidMotion,
		start_time: f32,
		end_time: f32,
		stop_at_penetration: bool,
	) -> std::result::Result<Option<ShapeCastHit>, Unsupported> {
		match other {
			TypedShape::Custom(other) => {
				if let Some(other) = other.downcast_ref::<Rect3d>() {
					todo!()
				} else {
					Err(Unsupported)
				}
			}
			TypedShape::Ball(_) => todo!(),
			TypedShape::Cuboid(_) => todo!(),
			TypedShape::Capsule(_) => todo!(),
			TypedShape::Segment(_) => todo!(),
			TypedShape::Triangle(_) => todo!(),
			TypedShape::TriMesh(_) => todo!(),
			TypedShape::Polyline(_) => todo!(),
			TypedShape::HalfSpace(_) => todo!(),
			TypedShape::HeightField(_) => todo!(),
			TypedShape::Compound(_) => todo!(),
			TypedShape::ConvexPolyhedron(_) => todo!(),
			TypedShape::Cylinder(_) => todo!(),
			TypedShape::Cone(_) => todo!(),
			TypedShape::RoundCuboid(_) => todo!(),
			TypedShape::RoundTriangle(_) => todo!(),
			TypedShape::RoundCylinder(_) => todo!(),
			TypedShape::RoundCone(_) => todo!(),
			TypedShape::RoundConvexPolyhedron(_) => todo!(),
		}
	}
}

/// Custom query dispatcher for `Rect3d`. Should be chained into `DefaultQueryDispatcher`.
pub struct Rect3dDispatcher;

impl QueryDispatcher for Rect3dDispatcher {
	fn intersection_test(
		&self,
		pos12: &Isometry<f32>,
		g1: &dyn Shape,
		g2: &dyn Shape,
	) -> std::result::Result<bool, Unsupported> {
		match (g1.as_typed_shape(), g2.as_typed_shape()) {
			(TypedShape::Custom(g1), g2) => {
				if let Some(rect) = g1.downcast_ref::<Rect3d>() {
					rect.intersection_test(pos12, g2)
				} else {
					Err(Unsupported)
				}
			}
			(g1, TypedShape::Custom(g2)) => {
				if let Some(rect) = g2.downcast_ref::<Rect3d>() {
					rect.intersection_test(&pos12.inverse(), g1)
				} else {
					Err(Unsupported)
				}
			}
			_ => Err(Unsupported),
		}
	}

	fn distance(
		&self,
		pos12: &Isometry<f32>,
		g1: &dyn Shape,
		g2: &dyn Shape,
	) -> std::result::Result<f32, Unsupported> {
		match (g1.as_typed_shape(), g2.as_typed_shape()) {
			(TypedShape::Custom(g1), g2) => {
				if let Some(rect) = g1.downcast_ref::<Rect3d>() {
					rect.distance(pos12, g2)
				} else {
					Err(Unsupported)
				}
			}
			(g1, TypedShape::Custom(g2)) => {
				if let Some(rect) = g2.downcast_ref::<Rect3d>() {
					rect.distance(&pos12.inverse(), g1)
				} else {
					Err(Unsupported)
				}
			}
			_ => Err(Unsupported),
		}
	}

	fn contact(
		&self,
		pos12: &Isometry<f32>,
		g1: &dyn Shape,
		g2: &dyn Shape,
		prediction: f32,
	) -> std::result::Result<Option<Contact>, Unsupported> {
		match (g1.as_typed_shape(), g2.as_typed_shape()) {
			(TypedShape::Custom(g1), g2) => {
				if let Some(rect) = g1.downcast_ref::<Rect3d>() {
					rect.contact(pos12, g2, prediction)
				} else {
					Err(Unsupported)
				}
			}
			(g1, TypedShape::Custom(g2)) => {
				if let Some(rect) = g2.downcast_ref::<Rect3d>() {
					rect.contact(&pos12.inverse(), g1, prediction)
						.map(|contact| {
							contact.map(|contact| Contact {
								point1: contact.point2,
								point2: contact.point1,
								normal1: contact.normal2,
								normal2: contact.normal1,
								dist: contact.dist,
							})
						})
				} else {
					Err(Unsupported)
				}
			}
			_ => Err(Unsupported),
		}
	}

	fn closest_points(
		&self,
		pos12: &Isometry<f32>,
		g1: &dyn Shape,
		g2: &dyn Shape,
		max_dist: f32,
	) -> std::result::Result<ClosestPoints, Unsupported> {
		match (g1.as_typed_shape(), g2.as_typed_shape()) {
			(TypedShape::Custom(g1), g2) => {
				if let Some(rect) = g1.downcast_ref::<Rect3d>() {
					rect.closest_points(pos12, g2, max_dist)
				} else {
					Err(Unsupported)
				}
			}
			(g1, TypedShape::Custom(g2)) => {
				if let Some(rect) = g2.downcast_ref::<Rect3d>() {
					rect.closest_points(&pos12.inverse(), g1, max_dist)
						.map(|pts| match pts {
							ClosestPoints::Intersecting => ClosestPoints::Intersecting,
							ClosestPoints::WithinMargin(p1, p2) => {
								ClosestPoints::WithinMargin(p2, p1)
							}
							ClosestPoints::Disjoint => ClosestPoints::Disjoint,
						})
				} else {
					Err(Unsupported)
				}
			}
			_ => Err(Unsupported),
		}
	}

	fn cast_shapes(
		&self,
		pos12: &Isometry<f32>,
		local_vel12: &Vector<f32>,
		g1: &dyn Shape,
		g2: &dyn Shape,
		options: ShapeCastOptions,
	) -> std::result::Result<Option<ShapeCastHit>, Unsupported> {
		match (g1.as_typed_shape(), g2.as_typed_shape()) {
			(TypedShape::Custom(g1), g2) => {
				if let Some(rect) = g1.downcast_ref::<Rect3d>() {
					rect.cast_shapes(pos12, local_vel12, g2, options)
				} else {
					Err(Unsupported)
				}
			}
			(g1, TypedShape::Custom(g2)) => {
				if let Some(rect) = g2.downcast_ref::<Rect3d>() {
					rect.cast_shapes(&pos12.inverse(), &-*local_vel12, g1, options)
						.map(|hit| {
							hit.map(|hit| ShapeCastHit {
								time_of_impact: hit.time_of_impact,
								witness1: hit.witness2,
								witness2: hit.witness1,
								normal1: hit.normal2,
								normal2: hit.normal1,
								status: hit.status,
							})
						})
				} else {
					Err(Unsupported)
				}
			}
			_ => Err(Unsupported),
		}
	}

	fn cast_shapes_nonlinear(
		&self,
		motion1: &NonlinearRigidMotion,
		g1: &dyn Shape,
		motion2: &NonlinearRigidMotion,
		g2: &dyn Shape,
		start_time: f32,
		end_time: f32,
		stop_at_penetration: bool,
	) -> std::result::Result<Option<ShapeCastHit>, Unsupported> {
		match (g1.as_typed_shape(), g2.as_typed_shape()) {
			(TypedShape::Custom(g1), g2) => {
				if let Some(rect) = g1.downcast_ref::<Rect3d>() {
					rect.cast_shapes_nonlinear(
						motion1,
						g2,
						motion2,
						start_time,
						end_time,
						stop_at_penetration,
					)
				} else {
					Err(Unsupported)
				}
			}
			(g1, TypedShape::Custom(g2)) => {
				if let Some(rect) = g2.downcast_ref::<Rect3d>() {
					rect.cast_shapes_nonlinear(
						motion2,
						g1,
						motion1,
						start_time,
						end_time,
						stop_at_penetration,
					)
						.map(|hit| {
							hit.map(|hit| ShapeCastHit {
								time_of_impact: hit.time_of_impact,
								witness1: hit.witness2,
								witness2: hit.witness1,
								normal1: hit.normal2,
								normal2: hit.normal1,
								status: hit.status,
							})
						})
				} else {
					Err(Unsupported)
				}
			}
			_ => Err(Unsupported),
		}
	}
}

#[cfg(test)]
mod tests {
    use super::*;
    use avian3d::parry::math::Isometry;
    use avian3d::parry::math::Point as P3;
    use avian3d::parry::math::Vector as V3;
    use avian3d::parry::query::{ClosestPoints, QueryDispatcher as _, Ray, ShapeCastOptions};
    use avian3d::parry::shape::{Ball, Shape as _};
    use parry2d::na::Vector2;
    use avian3d::parry::na;


    macro_rules! assert_approx_eq {
        ($a:expr, $b:expr) => {{
            let (a, b): (f32, f32) = ($a, $b);
            let diff = (a - b).abs();
            let eps: f32 = 1.0e-5;
            if diff > eps {
                panic!(
                    "assertion failed: |{} - {}| = {} > {} (left: {}, right: {})",
                    stringify!($a), stringify!($b), diff, eps, a, b
                );
            }
        }};
        ($a:expr, $b:expr, $eps:expr) => {{
            let (a, b, eps): (f32, f32, f32) = ($a, $b, $eps);
            let diff = (a - b).abs();
            if diff > eps {
                panic!(
                    "assertion failed: |{} - {}| = {} > {} (left: {}, right: {})",
                    stringify!($a), stringify!($b), diff, eps, a, b
                );
            }
        }};
    }

    fn rect() -> Rect3d {
        Rect3d::new(Vector2::new(1.0, 2.0))
    }


    // Run the same assertions over a set of (name, shape, pos) test cases.
    macro_rules! test_on_shapes {
        ($cases:expr, |$name:ident, $shape:ident, $pos:ident| $body:block) => {{
            for ($name, $shape, $pos) in $cases.into_iter() {
                $body
            }
        }};
    }

    use avian3d::parry::shape::Shape; // bring trait type for Box<dyn Shape>

    fn unimplemented_basic_shapes() -> Vec<(&'static str, Box<dyn Shape>, Isometry<f32>)> {
        use avian3d::parry::shape::{Segment as Segment3, Triangle as Triangle3, Polyline as Polyline3};
        let seg = Segment3 { a: P3::new(-0.5, 0.0, 0.0), b: P3::new(0.5, 0.0, 0.0) };
        let tri = Triangle3::new(P3::new(-0.5, 0.0, -0.5), P3::new(0.5, 0.0, -0.25), P3::new(0.0, 0.0, 0.5));
        let poly = Polyline3::new(vec![P3::new(-1.0, 0.0, 0.0), P3::new(0.0, 0.0, 1.0), P3::new(1.0, 0.0, 0.0)], None);
        let pos = Isometry::new(V3::new(0.0, 1.0, 0.0), na::zero());
        vec![
            ("segment", Box::new(seg), pos),
            ("triangle", Box::new(tri), pos),
            ("polyline", Box::new(poly), pos),
        ]
    }

    // These are intended to FAIL until functionality is implemented.
    #[test]
    fn distance_for_basic_shapes_expected() {
        let r = rect();
        test_on_shapes!(unimplemented_basic_shapes(), |_name, shape, pos| {
            let d = r
                .distance(&pos, (&*shape as &dyn Shape).as_typed_shape())
                .expect("distance should be supported once implemented");
            assert_approx_eq!(d, 1.0);
        });
    }

    #[test]
    fn contact_for_basic_shapes_expected() {
        let r = rect();
        test_on_shapes!(unimplemented_basic_shapes(), |_name, shape, pos| {
            let c = r
                .contact(&pos, (&*shape as &dyn Shape).as_typed_shape(), 1.1)
                .expect("contact query should not be Unsupported")
                .expect("expected contact within prediction once implemented");
            // Normals opposite, separation approx 1.0 for zero-thickness shapes.
            assert_approx_eq!(c.normal1.dot(&c.normal2), -1.0);
            assert_approx_eq!(c.dist, 1.0);
        });
    }

    #[test]
    fn closest_points_for_basic_shapes_expected() {
        let r = rect();
        test_on_shapes!(unimplemented_basic_shapes(), |_name, shape, pos| {
            // Large margin: expect WithinMargin
            let res = r
                .closest_points(&pos, (&*shape as &dyn Shape).as_typed_shape(), 1.5)
                .expect("closest_points should not be Unsupported");
            match res {
                ClosestPoints::WithinMargin(_, _) => {}
                other => panic!("expected WithinMargin, got {:?}", other),
            }
            // Small margin: expect Disjoint
            let res_small = r
                .closest_points(&pos, (&*shape as &dyn Shape).as_typed_shape(), 0.5)
                .expect("closest_points should not be Unsupported");
            match res_small {
                ClosestPoints::Disjoint => {}
                other => panic!("expected Disjoint with small margin, got {:?}", other),
            }
        });
    }

    #[test]
    fn cast_shapes_ball_expected_hit() {
        // Rect3d::cast_shapes(Ball) should report a hit for a downward-moving ball.
        let r = rect();
        let ball = Ball { radius: 0.5 };
        let pos = Isometry::new(V3::new(0.0, 3.0, 0.0), na::zero());
        let vel = V3::new(0.0, -1.0, 0.0);
        let hit = r
            .cast_shapes(&pos, &vel, (&ball as &dyn Shape).as_typed_shape(), ShapeCastOptions::default())
            .expect("cast_shapes should not be Unsupported");
        assert!(hit.is_some(), "expected a time-of-impact");
    }

    #[test]
    fn dispatcher_cast_shapes_ball_expected_hit() {
        let d = Rect3dDispatcher;
        let r = rect();
        let ball = Ball { radius: 0.5 };
        let pos = Isometry::new(V3::new(0.0, 3.0, 0.0), na::zero());
        let vel = V3::new(0.0, -1.0, 0.0);
        let hit = d
            .cast_shapes(&pos, &vel, &r as &dyn Shape, &ball as &dyn Shape, ShapeCastOptions::default())
            .expect("dispatcher cast_shapes should not be Unsupported");
        assert!(hit.is_some(), "expected a time-of-impact");
    }

    #[test]
    fn aabb_and_bounding_sphere() {
        let r = rect();
        let aabb = r.compute_local_aabb();
        assert_approx_eq!(aabb.mins.x, -1.0);
        assert_approx_eq!(aabb.mins.y, 0.0);
        assert_approx_eq!(aabb.mins.z, -2.0);
        assert_approx_eq!(aabb.maxs.x, 1.0);
        assert_approx_eq!(aabb.maxs.y, 0.0);
        assert_approx_eq!(aabb.maxs.z, 2.0);

        let bs = r.compute_local_bounding_sphere();
        // radius is the norm of half-extents in XZ
        assert_approx_eq!(bs.radius, (1.0f32 * 1.0 + 2.0 * 2.0).sqrt());
        assert_approx_eq!(bs.center.x, 0.0);
        assert_approx_eq!(bs.center.y, 0.0);
        assert_approx_eq!(bs.center.z, 0.0);
    }

    #[test]
    fn ray_cast_down_and_up_hits() {
        let r = rect();
        // From +Y side (forward), pointing toward -Y (backward).
        let hit_down = r.cast_local_ray_and_get_normal(&Ray { origin: P3::new(0.0, 2.0, 0.0), dir: V3::new(0.0, -1.0, 0.0) }, 100.0, false).unwrap();
        assert_approx_eq!(hit_down.time_of_impact, 2.0);
        // Implementation returns -Y normal when ray points toward -Y (backward).
        assert_approx_eq!(hit_down.normal.x, 0.0);
        assert_approx_eq!(hit_down.normal.y, -1.0);
        assert_approx_eq!(hit_down.normal.z, 0.0);

        // From -Y side (backward), pointing toward +Y (forward).
        let hit_up = r.cast_local_ray_and_get_normal(&Ray { origin: P3::new(0.0, -3.0, 0.0), dir: V3::new(0.0, 1.0, 0.0) }, 100.0, false).unwrap();
        assert_approx_eq!(hit_up.time_of_impact, 3.0);
        assert_approx_eq!(hit_up.normal.y, 1.0);
    }

    #[test]
    fn ray_cast_parallel_none() {
        let r = rect();
        // Origin above, direction up -> away from plane: should be None.
        let none = r.cast_local_ray_and_get_normal(&Ray { origin: P3::new(0.5, 1.0, 0.5), dir: V3::new(0.0, 1.0, 0.0) }, 100.0, false);
        assert!(none.is_none());
        // Origin on plane, dir parallel -> treated as no hit.
        let none2 = r.cast_local_ray_and_get_normal(&Ray { origin: P3::new(0.0, 0.0, 0.0), dir: V3::new(1.0, 0.0, 0.0) }, 100.0, false);
        assert!(none2.is_none());
    }

    #[test]
    fn point_query_projects_to_plane() {
        let r = rect();
        // Point above, inside XZ extents.
        let proj = r.project_local_point(&P3::new(0.25, 3.0, 0.5), false);
        assert!(!proj.is_inside);
        assert_approx_eq!(proj.point.x, 0.25);
        assert_approx_eq!(proj.point.y, 0.0);
        assert_approx_eq!(proj.point.z, 0.5);

        // Point outside on X, should clamp to edge at x=1.0
        let proj2 = r.project_local_point(&P3::new(5.0, 0.0, 0.0), false);
        assert_approx_eq!(proj2.point.x, 1.0);
        assert_approx_eq!(proj2.point.z, 0.0);
    }

    #[test]
    fn support_map_axes() {
        let r = rect();
        // +X
        let p = r.local_support_point(&Vector3::new(1.0, 0.0, 0.0));
        assert_approx_eq!(p.x, 1.0);
        assert_approx_eq!(p.y, 0.0);
        assert_approx_eq!(p.z.abs(), 2.0);
        // -Z
        let p = r.local_support_point(&Vector3::new(0.0, 0.0, -1.0));
        assert_approx_eq!(p.z, -2.0);
        assert_approx_eq!(p.y, 0.0);
    }

    #[test]
    fn shape_flags_and_scaling() {
        let r = rect();
        assert!(r.is_convex());
        assert_eq!(r.shape_type(), ShapeType::Custom);
        assert_approx_eq!(r.ccd_thickness(), 0.0);
        assert_approx_eq!(r.ccd_angular_thickness(), std::f32::consts::FRAC_PI_2);

        let scaled = r.scale_dyn(&V3::new(2.0, 5.0, 0.5), 0).unwrap();
        let scaled = scaled.as_typed_shape();
        // Extract half-extents via aabb
        if let TypedShape::Custom(custom) = scaled {
            let r2 = custom.downcast_ref::<Rect3d>().unwrap();
            let aabb = r2.compute_local_aabb();
            assert_approx_eq!(aabb.maxs.x, 2.0);
            assert_approx_eq!(aabb.maxs.z, 1.0);
        } else {
            panic!("expected custom scaled Rect3d");
        }
    }

    #[test]
    fn distance_to_ball_above_and_outside() {
        let r = rect();
        let ball = Ball { radius: 2.0 };
        let pos = Isometry::new(V3::new(0.0, 5.0, 0.0), na::zero());
        let d = r.distance(&pos, (&ball as &dyn Shape).as_typed_shape()).unwrap();
        assert_approx_eq!(d, 3.0);

        // On plane but far along +X: rectangle edge at x=1.0, center at x=10.0, radius 1.0 => dist 8.0
        let ball2 = Ball { radius: 1.0 };
        let pos2 = Isometry::new(V3::new(10.0, 0.0, 0.0), na::zero());
        let d2 = r.distance(&pos2, (&ball2 as &dyn Shape).as_typed_shape()).unwrap();
        assert_approx_eq!(d2, 9.0 - 1.0);
    }

    // --- Additional tests for not-yet-implemented or partially implemented areas ---

    #[test]
    fn intersection_test_with_ball_true_and_false() {
        let r = rect();
        let ball = Ball { radius: 1.0 };
        // Intersecting: center at y=0.5 (radius 1.0)
        let pos_inter = Isometry::new(V3::new(0.0, 0.5, 0.0), na::zero());
        let it = r.intersection_test(&pos_inter, (&ball as &dyn Shape).as_typed_shape()).unwrap();
        assert!(it, "expected intersecting when ball center y < radius");
        // Not intersecting: center at y=3.0
        let pos_dis = Isometry::new(V3::new(0.0, 3.0, 0.0), na::zero());
        let it2 = r.intersection_test(&pos_dis, (&ball as &dyn Shape).as_typed_shape()).unwrap();
        assert!(!it2, "expected disjoint when ball center y > radius and inside XZ");
    }

    #[test]
    fn contact_with_ball_inside_prediction() {
        let r = rect();
        let ball = Ball { radius: 1.0 };
        // center above plane at y=1.5 => dist = 0.5
        let pos = Isometry::new(V3::new(0.0, 1.5, 0.0), na::zero());
        let c = r
            .contact(&pos, (&ball as &dyn Shape).as_typed_shape(), 1.0)
            .unwrap()
            .expect("expected contact within prediction");
        assert_approx_eq!(c.dist, 0.5);
        // Normals should point +Y for rect and -Y for ball in this configuration
        assert_approx_eq!(c.normal1.y, 1.0);
        assert_approx_eq!(c.normal2.y, -1.0);
    }

    #[test]
    fn feature_normals_basic() {
        let r = rect();
        // Face normal is +Y
        let n = r.feature_normal_at_point(FeatureId::Face(0), &P3::origin()).unwrap();
        assert_approx_eq!(n.y, 1.0);
        // An edge normal, e.g., edge 0 = +Z
        let n0 = r.feature_normal_at_point(FeatureId::Edge(0), &P3::origin()).unwrap();
        assert_approx_eq!(n0.z, 1.0);
    }

    #[test]
    fn dispatcher_contact_swaps_points_and_normals() {
        let r = rect();
        let ball = Ball { radius: 1.0 };
        let pos = Isometry::new(V3::new(0.0, 1.25, 0.0), na::zero());
        let d = Rect3dDispatcher;
        let c12 = d
            .contact(&pos, &r as &dyn Shape, &ball as &dyn Shape, 1.0)
            .unwrap()
            .expect("expected contact rect-ball");
        let c21 = d
            .contact(&pos.inverse(), &ball as &dyn Shape, &r as &dyn Shape, 1.0)
            .unwrap()
            .expect("expected contact ball-rect");
        // Swapping should invert witness order and normals
        assert_approx_eq!(c12.point1.coords.x, c21.point2.coords.x);
        assert_approx_eq!(c12.point1.coords.z, c21.point2.coords.z);
        assert_approx_eq!(c12.normal1.y, c21.normal2.y);
        assert_approx_eq!(c12.dist, c21.dist);
    }

    #[test]
    fn project_shape_ball_distance_and_type() {
        use avian3d::parry::math::Isometry as Iso3;
        let origin = Iso3::identity();
        let ball = Ball { radius: 0.75 };
        let pos = Iso3::new(V3::new(0.0, 3.0, 0.0), na::zero());
        let (shape2d, _iso2, dist) = super::project_shape_onto_plane(&origin, &ball, &pos);
        assert_approx_eq!(dist, 3.0);
        match shape2d.as_typed_shape() {
            parry2d::shape::TypedShape::Ball(b2) => assert_approx_eq!(b2.radius, 0.75),
            other => panic!("expected 2D ball, got {:?}", other),
        }
    }

    #[test]
    fn project_shape_segment_not_degenerate() {
        use avian3d::parry::math::Isometry as Iso3;
        use avian3d::parry::shape::Segment as Segment3;
        let origin = Iso3::identity();
        let seg = Segment3 {
            a: P3::new(0.0, 0.0, -1.0),
            b: P3::new(0.0, 0.0, 1.0),
        };
        let pos = Iso3::identity();
        let (shape2d, _iso2, dist) = super::project_shape_onto_plane(&origin, &seg, &pos);
        assert_approx_eq!(dist, 0.0);
        match shape2d.as_typed_shape() {
            parry2d::shape::TypedShape::Segment(s2) => {
                // Expect different endpoints; current implementation projects 'a' twice (bug), so this will fail until fixed.
                assert!(s2.a != s2.b, "projected segment endpoints should differ");
            }
            other => panic!("expected 2D segment, got {:?}", other),
        }
    }

    #[test]
    fn distance_to_cuboid_centered_above() {
        use avian3d::parry::shape::Cuboid as Cuboid3;
        let r = rect();
        let cuboid = Cuboid3::new(Vector3::new(0.1, 0.5, 0.1));
        let pos = Isometry::new(V3::new(0.0, 2.0, 0.0), na::zero());


        let d = r
            .distance(&pos, (&cuboid as &dyn Shape).as_typed_shape())
            .expect("distance should not be Unsupported for cuboid");
        // Expected distance along Y: 2.0 - 0.5 = 1.5 (inside XZ extents)
        assert_approx_eq!(d, 1.5);
    }



    #[test]
    fn closest_points_within_margin_expected() {
        let r = rect();
        let ball = Ball { radius: 1.0 };
        // Center at y=1.5 in front of plane, margin 1.0 -> distance to plane is 1.5 - 1.0 = 0.5 <= margin
        let pos = Isometry::new(V3::new(0.0, 1.5, 0.0), na::zero());
        let res = r.closest_points(&pos, (&ball as &dyn Shape).as_typed_shape(), 1.0).unwrap();
        match res {
            ClosestPoints::WithinMargin(p1, p2) => { dbg!(p1, p2); },
            other => panic!("expected WithinMargin, got {:?}", other),
        }
    }

    #[test]
    fn dispatcher_distance_is_symmetric() {
        let r = rect();
        let ball = Ball { radius: 2.0 };
        let pos = Isometry::new(V3::new(0.5, 3.0, -0.5), na::zero());
        let dispatcher = Rect3dDispatcher;
        let d1 = dispatcher
            .distance(&pos, &r as &dyn Shape, &ball as &dyn Shape)
            .unwrap();
        let d2 = dispatcher
            .distance(&pos.inverse(), &ball as &dyn Shape, &r as &dyn Shape)
            .unwrap();
        assert_approx_eq!(d1, d2);
    }
}

