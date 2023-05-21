//! Data structures that can be deserialized from a parameter list.

use std::{collections::HashMap, str::FromStr};

use crate::{
    param::{Param, ParamList},
    Error, Result,
};

/// The coordinate system.
#[derive(Debug, Default, Eq, PartialEq)]
pub enum CoordinateSystem {
    /// Translate the scene so that the camera is at the origin.
    #[default]
    CameraWorld,
    /// Use camera space.
    Camera,
    /// Uses world space.
    World,
}

impl FromStr for CoordinateSystem {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "cameraworld" => Ok(CoordinateSystem::CameraWorld),
            "camera" => Ok(CoordinateSystem::Camera),
            "world" => Ok(CoordinateSystem::World),
            _ => Err(Error::UnknownCoordinateSystem),
        }
    }
}

/// Scene-wide rendering options.
#[derive(Debug)]
pub struct Options {
    /// Forces all pixel samples to be through the center of the pixel area.
    pub disable_pixel_jitter: bool,
    /// Forces point sampling at the finest MIP level for all texture lookups.
    pub disable_texture_filtering: bool,
    /// Forces all samples within each pixel to sample the same wavelengths.
    pub disable_wavelength_jitter: bool,
    /// Global scale factor applied to triangle edge lengths before evaluating
    /// the edge length test for refinement when applying displacement mapping.
    pub displacement_edge_scale: f32,
    /// Specifies the filename of an image to use when computing mean squared
    /// error versus the number of pixel samples taken
    pub mse_reference_image: Option<String>,
    /// Filename for per-sample mean squared error results.
    pub mse_reference_out: Option<String>,
    /// Specifies the coordinate system to use for rendering computation.
    pub render_coord_sys: CoordinateSystem,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            disable_pixel_jitter: false,
            disable_texture_filtering: false,
            disable_wavelength_jitter: false,
            displacement_edge_scale: 1.0,
            mse_reference_image: None,
            mse_reference_out: None,
            render_coord_sys: CoordinateSystem::CameraWorld,
        }
    }
}

impl Options {
    pub fn apply(&mut self, _option: Param) -> Result<()> {
        Ok(())
    }
}

#[derive(Default, Debug)]
pub enum FilmType {
    /// Stores RGB images using the current color space when the [Film] directive is encountered.
    #[default]
    Rgb,
    /// In addition to RGB, stores multiple additional channels that encode information about the visible geometry in each pixel.
    GBuffer {
        /// Geometric data is stored in camera space by default.
        /// Alternatively, "world" can be specified for world space.
        coordinate_system: String,
    },
    /// Stores a discretized spectral distribution at each pixel, in addition to RGB.
    Spectral {
        /// Number of buckets that the spectral range is discretized into.
        nbuckets: i32,
        /// Minimum wavelength for spectral range.
        lambda_min: f32,
        /// Maximum wavelength for spectral range.
        lambda_max: f32,
    },
}

/// Film specifies the characteristics of the image being generated by the renderer.
#[derive(Debug)]
pub struct Film {
    /// The number of pixels in the x direction.
    pub xresolution: i32,
    /// The number of pixels in the y direction.
    pub yresolution: i32,
    /// The sub-region of the image to render.
    pub crop_window: [f32; 4],
    /// Diagonal length of the film, in mm.
    pub diagonal: f32,
    /// The output filename.
    pub filename: String,
    /// Whether 16-bit floating point values (as opposed to 32-bit
    /// floating point values) should be used when saving images in OpenEXR format.
    pub save_fp16: bool,
    /// Film sensitivity to light; final pixel values are scaled by the "iso" value divided by 100.
    pub iso: f32,
    /// If non-zero, this gives a temperature in degrees kelvin
    /// that is used as the reference color temperature used for whitebalancing.
    pub white_balance: f32,
    /// Characterizes the sensor's response for red, green, and blue colors.
    /// The default corresponds to using the CIE 1931 spectral response curves.
    pub sensor: String,
    /// Image sample values with luminance greater than this value are clamped to have this luminance.
    pub max_component_value: f32,
    /// Film type.
    pub ty: FilmType,
}

impl Default for Film {
    fn default() -> Self {
        Self {
            xresolution: 1280,
            yresolution: 720,
            crop_window: [0.0, 1.0, 0.0, 1.0],
            diagonal: 35.0,
            filename: String::from("pbrt.exr"),
            save_fp16: true,
            iso: 100.0,
            white_balance: 0.0,
            sensor: String::from("cie1931"),
            max_component_value: f32::MAX,
            ty: FilmType::Rgb,
        }
    }
}

impl Film {
    pub fn new(ty: &str, params: ParamList) -> Result<Film> {
        let ty = match ty {
            "rgb" => FilmType::Rgb,
            "gbuffer" => FilmType::GBuffer {
                coordinate_system: params
                    .string("coordinatesystem")
                    .unwrap_or("camera")
                    .to_owned(),
            },
            "spectral" => FilmType::Spectral {
                nbuckets: params.integer("nbuckets", 16),
                lambda_min: params.float("lambdamin", 360.0),
                lambda_max: params.float("lambdamax", 830.0),
            },
            _ => unimplemented!(),
        };

        let film = Film {
            xresolution: params.integer("xresolution", 1280),
            yresolution: params.integer("yresolution", 720),
            crop_window: params
                .floats("cropwindow")
                .unwrap_or(&[0.0, 1.0, 0.0, 1.0])
                .try_into()
                .map_err(|_| Error::ParseSlice)?,
            diagonal: params.float("diagonal", 35.0),
            filename: params.string("filename").unwrap_or("pbrt.exr").to_owned(),
            save_fp16: params.boolean("savefp16").unwrap_or(true),
            iso: params.float("iso", 100.0),
            white_balance: params.float("whitebalance", 0.0),
            sensor: params.string("sensor").unwrap_or("cie1931").to_owned(),
            max_component_value: params.float("maxcomponentvalue", f32::MAX),
            ty,
        };

        Ok(film)
    }
}

#[derive(Debug)]
pub enum Camera {
    Orthographic {
        /// The time at which the virtual camera shutter opens.
        shutter_open: f32,
        /// The time at which the virtual camera shutter closes.
        shutter_close: f32,
    },
    Perspective {
        /// The time at which the virtual camera shutter opens.
        shutter_open: f32,
        /// The time at which the virtual camera shutter closes.
        shutter_close: f32,
        /// Specifies the field of view for the perspective camera.
        fov: f32,
    },
    /// The `RealisticCamera` simulates imaging from light rays passing through complex lens systems.
    Realistic {
        /// The time at which the virtual camera shutter opens.
        shutter_open: f32,
        /// The time at which the virtual camera shutter closes.
        shutter_close: f32,
        /// Specifies the name of a lens description file that gives the collection of lens elements in the lens system.
        lensfile: Option<String>,
        /// Diameter of the lens system's aperture, specified in mm. The smaller the aperture,
        /// the less light reaches the film plane, but the greater the range of distances that are in focus.
        aperture_diameter: f32,
        /// Distance in meters at which the lens system is focused.
        focus_distance: f32,
        /// Allows specifying the shape of the camera aperture, which is circular by default.
        /// The values of "gaussian", "square", "pentagon", and "star" are associated with built-in aperture shapes;
        /// other values are interpreted as filenames specifying an image to be used to specify the shape.
        aperture: Option<String>,
    },
    /// The SphericalCamera captures light arriving at the camera from all directions.
    Spherical {
        /// The time at which the virtual camera shutter opens.
        shutter_open: f32,
        /// The time at which the virtual camera shutter closes.
        shutter_close: f32,
        /// By default, an area-preserving mapping based on an octahedral encoding of the unit sphere is used.
        /// Alternatively, an equirectangular mapping can be specified using "equirectangular".
        mapping: String,
    },
}

impl Camera {
    pub fn new(ty: &str, params: ParamList) -> Result<Camera> {
        // Two parameters that set the camera's shutter open times are common to all cameras in pbrt.
        let shutter_open = params.float("shutteropen", 0.0);
        let shutter_close = params.float("shutterclose", 1.0);

        let camera = match ty {
            "orthographic" => Camera::Orthographic {
                shutter_open,
                shutter_close,
            },
            "perspective" => Camera::Perspective {
                shutter_open,
                shutter_close,
                fov: params.float("fov", 90.0),
            },
            "realistic" => Camera::Realistic {
                shutter_open,
                shutter_close,
                lensfile: params.string("lensfile").map(|str| str.to_string()),
                aperture_diameter: params.float("aperturediameter", 1.0),
                focus_distance: params.float("focusdistance", 10.0),
                aperture: params.string("aperture").map(|str| str.to_string()),
            },
            "spherical" => Camera::Spherical {
                shutter_open,
                shutter_close,
                mapping: params.string("mapping").unwrap_or("equalarea").to_string(),
            },
            _ => return Err(Error::InvalidCameraType),
        };

        Ok(camera)
    }
}

/// The integrator implements the light transport algorithm that computes radiance
/// arriving at the film plane from surfaces and participating media in the scene.
///
/// Many of these integrators are present only for pedagogical purposes or for use in debugging
/// more complex integrators through computing images using much simpler integration algorithms.
/// For rendering high quality images, one should almost always use one of `bdpt`, `mlt`, `sppm`, or `volpath`.
#[derive(Debug)]
pub enum Integrator {
    /// Ambient occlusion (accessibility over the hemisphere).
    AmbientOcclusion,
    /// Bidirectional path tracing.
    Bdpt,
    /// Path tracing starting from the light sources.
    LightPath,
    /// Metropolis light transport using bidirectional path tracing.
    Mlt,
    /// Path tracing.
    Path,
    /// Rendering using a simple random walk without any explicit light sampling.
    RandomWalk,
    /// Path tracing with very basic sampling algorithms.
    SimplePath,
    /// Volumetric path tracing with very basic sampling algorithms.
    SimpleVolPath,
    /// Stochastic progressive photon mapping
    Sppm,
    /// Volumetric path tracing.
    VolPath {
        /// Maximum length of a light-carrying path sampled by the integrator.
        max_depth: i32,
    },
}

impl Integrator {
    pub fn new(ty: &str, params: ParamList) -> Result<Integrator> {
        let integ = match ty {
            "volpath" => Integrator::VolPath {
                max_depth: params.integer("maxdepth", 5),
            },
            _ => unimplemented!(),
        };

        Ok(integ)
    }
}

#[derive(Debug, Default)]
pub enum BvhSplitMethod {
    /// Denotes the surface area heuristic.
    #[default]
    Sah,
    /// Splits each node at its midpoint along the split axis.
    Middle,
    /// Splits the current group of primitives into two equal-sized sets
    Equal,
    /// Selects the HLBVH algorithm, which parallelizes well.
    Hlbvh,
}

#[derive(Debug)]
pub enum Accelerator {
    Bvh {
        /// Maximum number of primitives to allow in a node in the tree.
        max_node_prims: i32,
        /// Method to use to partition the primitives when building the tree.
        split_method: BvhSplitMethod,
    },
    KdTree {
        /// The value of the cost function that estimates the expected cost of
        /// performing a ray-object intersection, for use in building the kd-tree.
        intersect_cost: i32,
        /// Estimated cost for traversing a ray through a kd-tree node.
        traversal_cost: i32,
        /// "Bonus" factor for kd-tree nodes that represent empty space.
        empty_bonus: f32,
        /// Maximum number of primitives to store in kd-tree node.
        max_prims: i32,
        /// Maximum depth of the kd-tree. If negative, the kd-tree chooses a maximum depth
        /// based on the number of primitives to be stored in it.
        max_depth: i32,
    },
}

impl Accelerator {
    pub fn new(ty: &str, params: ParamList) -> Result<Accelerator> {
        let acc = match ty {
            "bvh" => Accelerator::Bvh {
                max_node_prims: params.integer("maxnodeprims", 4),
                split_method: match params.string("splitmethod").unwrap_or("sah") {
                    "sah" => BvhSplitMethod::Sah,
                    "middle" => BvhSplitMethod::Middle,
                    "equal" => BvhSplitMethod::Equal,
                    "hlbvh" => BvhSplitMethod::Hlbvh,
                    _ => return Err(Error::InvalidString),
                },
            },
            "kdtree" => Accelerator::KdTree {
                intersect_cost: params.integer("intersectcost", 5),
                traversal_cost: params.integer("traversalcost", 1),
                empty_bonus: params.float("emptybonus", 0.5),
                max_prims: params.integer("maxprims", 1),
                max_depth: params.integer("maxdepth", -1),
            },
            _ => return Err(Error::InvalidString),
        };

        Ok(acc)
    }
}

// The Sampler generates samples for the image, time, lens, and Monte Carlo integration.
#[derive(Debug, Default)]
pub enum Sampler {
    Halton,
    Independent,
    PaddedSobol,
    Sobol,
    Stratified,
    #[default]
    ZSobol,
}

impl Sampler {
    pub fn new(ty: &str, _params: ParamList) -> Result<Sampler> {
        let sampler = match ty {
            "halton" => Sampler::Halton,
            "independent" => Sampler::Independent,
            "paddedsobol" => Sampler::PaddedSobol,
            "sobol" => Sampler::Sobol,
            "stratified" => Sampler::Stratified,
            "zsobol" => Sampler::ZSobol,
            _ => return Err(Error::InvalidObjectType),
        };

        Ok(sampler)
    }
}

/// Light sources cast illumination in the scene.
#[derive(Debug)]
pub enum Light {
    /// The "distant" light source represents a directional light source "at infinity";
    /// In other words, it illuminates the scene with light arriving from a single direction.
    Distant,
    GonioPhotometric,
    /// The "infinite" light represents an infinitely far away light source that
    /// potentially casts illumination from all directions.
    Infinite {
        /// The environment map to use for the infinite area light.
        /// If no filename is provided, the light will emit the same amount of radiance from every direction.
        filename: Option<String>,
        /// The spectral distribution of emission from the light.
        l: Option<[f32; 3]>,
    },
    Point,
    Projection,
    Spot,
}

impl Light {
    pub fn new(ty: &str, params: ParamList) -> Result<Light> {
        let light = match ty {
            "distant" => Light::Distant,
            "goniometric" => Light::GonioPhotometric,
            "infinite" => Light::Infinite {
                filename: params.string("filename").map(|f| f.to_owned()),
                l: match params.floats("L") {
                    Some(f) => Some(f.try_into().map_err(|_| Error::ParseSlice)?),
                    None => None,
                },
            },
            "point" => Light::Point,
            "projection" => Light::Projection,
            "spot" => Light::Spot,
            _ => unimplemented!(),
        };

        Ok(light)
    }
}

#[derive(Debug)]
pub enum TextureType {
    Float,
    Spectrum,
}

#[derive(Debug)]
pub struct Texture {
    pub name: String,
    pub ty: TextureType,
    pub class: String,
}

impl Texture {
    pub fn new(name: &str, ty: &str, class: &str, _params: ParamList) -> Result<Texture> {
        let ty = match ty {
            "spectrum" => TextureType::Spectrum,
            "float" => TextureType::Float,
            _ => return Err(Error::InvalidObjectType),
        };

        // TODO: Parse parameters.

        Ok(Texture {
            name: name.to_string(),
            ty,
            class: class.to_string(),
        })
    }
}

/// Materials specify the light scattering properties of surfaces in the scene.
pub struct Material {
    pub ty: String,
}

impl Material {
    pub fn new(
        name: &str,
        _params: ParamList,
        _texture_map: &HashMap<String, usize>,
    ) -> Result<Material> {
        // Parameters to materials are distinctive in that textures can be used to
        // specify spatially-varying values for the parameters.

        // TODO: Parse material parameters.

        Ok(Material {
            ty: name.to_string(),
        })
    }
}

#[derive(Debug)]
pub enum Shape {
    /// The "cylinder" is always oriented along the z axis.
    Cylinder {
        alpha: f32,
        /// The cylinder's radius.
        radius: f32,
        /// The height of the cylinder's bottom along the z axis.
        zmin: f32,
        /// The height of the cylinder's top along the z axis.
        zmax: f32,
        /// The maximum extent of the cylinder in phi (in spherical coordinates).
        phimax: f32,
    },
    /// The "disk" is perpendicular to the z axis in the xy plane, with its object space center at x=0 and y=0.
    Disk {
        alpha: f32,
        /// The position of the disk along the z axis.
        height: f32,
        /// The outer radius of the disk.
        radius: f32,
        /// The inner radius of the disk (if nonzero, the disk is an annulus).
        innerradius: f32,
        /// The maximum extent of the disk in phi (in spherical coordinates).
        phimax: f32,
    },
    /// Spheres are always at the origin in object space.
    Sphere {
        alpha: f32,
        /// The sphere's radius.
        radius: f32,
        /// The height of the lower clipping plane along the z axis.
        zmin: f32,
        /// The height of the upper clipping plane along the z axis.
        zmax: f32,
        /// The maximum extent of the sphere in phi (in spherical coordinates).
        phimax: f32,
    },
    /// A triangle mesh is defined by the "trianglemesh" shape.
    TriangleMesh {
        alpha: f32,
        /// The mesh's topology is defined by the `indices` parameter,
        /// which is an array of integer indices into the vertex arrays.
        indices: Vec<i32>,
        /// Each successive triplet of indices defines the offsets to
        /// the three vertices of one triangle; thus, the length of the
        /// indices array must be a multiple of three.
        positions: Vec<f32>,
        /// Per-vertex normals.
        normals: Vec<f32>,
        /// Per-vertex tangents.
        tangents: Vec<f32>,
        /// Per-vertex texture coordinates.
        uvs: Vec<f32>,
    },
}

impl Shape {
    pub fn new(ty: &str, params: ParamList) -> Result<Self> {
        // All shapes take an optional "alpha" parameter that can be
        // used to define a mask that cuts away regions of a surface.
        let alpha = params.float("alpha", 1.0);

        let shape = match ty {
            "cylinder" => Shape::Cylinder {
                alpha,
                radius: params.float("radius", 1.0),
                zmin: params.float("zmin", -1.0),
                zmax: params.float("zmax", 1.0),
                phimax: params.float("phimax", 360.0),
            },
            "disk" => Shape::Disk {
                alpha,
                height: params.float("height", 0.0),
                radius: params.float("radius", 1.0),
                innerradius: params.float("innerradius", 0.0),
                phimax: params.float("phimax", 360.0),
            },
            "sphere" => {
                let radius = params.float("radius", 1.0);

                let zmin = params.float("zmin", -radius);
                let zmax = params.float("zmax", radius);

                Shape::Sphere {
                    alpha,
                    radius,
                    zmin,
                    zmax,
                    phimax: params.float("phimax", 360.0),
                }
            }
            "trianglemesh" => {
                // TODO: Positions and indices are required, return error if not provided.
                let indices = Vec::from(params.integers("indices").unwrap_or_default());
                debug_assert_eq!(indices.len() % 3, 0);

                let positions = Vec::from(params.floats("P").unwrap_or_default());

                let normals = Vec::from(params.floats("N").unwrap_or_default());
                let tangents = Vec::from(params.floats("S").unwrap_or_default());

                let uvs = Vec::from(params.floats("uv").unwrap_or_default());

                Shape::TriangleMesh {
                    alpha,
                    indices,
                    positions,
                    normals,
                    uvs,
                    tangents,
                }
            }
            _ => unimplemented!(),
        };

        Ok(shape)
    }
}

#[derive(Debug, Default)]
pub struct Medium {}

impl Medium {
    pub fn new(_params: ParamList) -> Result<Self> {
        // TODO: Handle medium object initialization.
        Ok(Medium {})
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_coord_sys() {
        assert_eq!(
            CoordinateSystem::from_str("cameraworld"),
            Ok(CoordinateSystem::CameraWorld)
        );
        assert_eq!(
            CoordinateSystem::from_str("camera"),
            Ok(CoordinateSystem::Camera)
        );
        assert_eq!(
            CoordinateSystem::from_str("world"),
            Ok(CoordinateSystem::World)
        );

        assert!(CoordinateSystem::from_str("").is_err());
        assert!(CoordinateSystem::from_str("foo").is_err());
    }
}