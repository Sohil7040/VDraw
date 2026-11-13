use crate::config::LaserProductionConfig;
use crate::estimator::{trapezoidal_move_time, CamJobEstimation};
use crate::kerf::apply_kerf_compensation;
use crate::operation::{classify_color_to_mode, LaserMode, LayerLaserSettings};
use crate::ordering::{calculate_containment_depths, optimize_cutting_order, CamContour};
use crate::postprocessor::{post_process, MachineConfig, MachineFlavor, PostProcessError};
use core_doc::Document;
use core_geom::Point2;

/// A single discrete toolpath segment ready for machining or preview.
#[derive(Debug, Clone, PartialEq)]
pub struct Toolpath {
    pub shape_id: u64,
    pub layer_name: String,
    pub mode: LaserMode,
    pub points: Vec<Point2>,
    pub closed: bool,
    pub speed_mm_s: f64,
    pub power_pct: f64,
    pub passes: u32,
    pub air_assist: bool,
    pub cut_length_mm: f64,
    pub sequence_index: usize,
}

/// A complete prepared laser machining job plan.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CamJobPlan {
    pub toolpaths: Vec<Toolpath>,
    /// Laser-off rapid travel moves: (from_pt, to_pt)
    pub rapid_moves: Vec<(Point2, Point2)>,
    pub total_cut_length_mm: f64,
    pub total_rapid_length_mm: f64,
    pub estimated_duration_s: f64,
    /// Detailed motion-profiled job estimation
    pub estimation: CamJobEstimation,
}

impl CamJobPlan {
    /// Generate standard G-Code for CNC laser cutters (GRBL, Marlin, Smoothie, LinuxCNC).
    pub fn export_gcode(&self) -> String {
        let config = MachineConfig {
            flavor: MachineFlavor::GenericGcode,
            ..Default::default()
        };
        post_process(self, &config).unwrap_or_default()
    }

    /// Generate HPGL / PLT format for vinyl plotters and standard laser systems.
    pub fn export_plt(&self) -> String {
        let config = MachineConfig {
            flavor: MachineFlavor::Hpgl,
            ..Default::default()
        };
        post_process(self, &config).unwrap_or_default()
    }

    /// Export machine instructions in specific machine controller dialect (GRBL, Marlin, Generic, HPGL).
    pub fn export_dialect(&self, config: &MachineConfig) -> Result<String, PostProcessError> {
        post_process(self, config)
    }
}

/// Generate a production-ready machining plan from document geometry and laser settings.
pub fn generate_job_plan(doc: &Document, config: &LaserProductionConfig) -> CamJobPlan {
    let Some(page) = doc.active_page() else {
        return CamJobPlan::default();
    };

    let mut raw_contours = Vec::new();

    // 1. Extract contours from visible layers and shapes
    for (layer_idx, layer) in page.layers.iter().enumerate() {
        if !layer.visible {
            continue;
        }

        let default_layer_settings = config
            .layer_settings
            .get(&layer.name)
            .cloned()
            .unwrap_or_else(|| {
                // Auto-classify layer by name
                let name_lower = layer.name.to_lowercase();
                let color_idx = (layer_idx % 16) as u8;
                if name_lower.contains("engrave") || name_lower.contains("scan") {
                    LayerLaserSettings::new_engrave(180.0, 40.0, color_idx)
                } else if name_lower.contains("score") || name_lower.contains("mark") {
                    LayerLaserSettings::new_score(60.0, 30.0, color_idx)
                } else if name_lower.contains("guide") || name_lower.contains("ignore") {
                    LayerLaserSettings {
                        mode: LaserMode::Ignore,
                        color_index: color_idx,
                        ..Default::default()
                    }
                } else {
                    LayerLaserSettings::new_cut(25.0, 80.0, color_idx)
                }
            });

        for shape in &layer.shapes {
            if !shape.visible {
                continue;
            }

            // Check object-level override first, then layer settings, then auto-color classification
            let settings = if let Some(override_s) = config.shape_overrides.get(&shape.id.0) {
                override_s.clone()
            } else if config.auto_color_classify {
                if let Some(stroke) = &shape.style.stroke {
                    let rgb = [stroke.color.r, stroke.color.g, stroke.color.b];
                    let mode = classify_color_to_mode(rgb);
                    LayerLaserSettings {
                        mode,
                        ..default_layer_settings.clone()
                    }
                } else {
                    default_layer_settings.clone()
                }
            } else {
                default_layer_settings.clone()
            };

            if settings.mode == LaserMode::Ignore {
                continue;
            }

            let poly = shape.to_polycurve();
            let tolerance_mm = 0.05; // 0.05mm chord error bound for machine curves

            for subpath in &poly.subpaths {
                let local_pts = subpath.flatten(tolerance_mm);
                if local_pts.len() >= 2 {
                    let world_pts: Vec<Point2> = local_pts
                        .iter()
                        .map(|&p| shape.transform.transform_point(p))
                        .collect();
                    let contour = CamContour::new(
                        world_pts,
                        subpath.closed,
                        shape.id.0,
                        layer.name.clone(),
                        settings.mode,
                    );
                    raw_contours.push(contour);
                }
            }
        }
    }

    if raw_contours.is_empty() {
        return CamJobPlan::default();
    }

    // 2. Classify containment depths and inner holes
    calculate_containment_depths(&mut raw_contours);

    // 3. Apply kerf compensation if configured
    if config.kerf_mode != crate::operation::KerfMode::Center && config.kerf > 1e-4 {
        for c in &mut raw_contours {
            if c.closed && c.mode.is_cut() {
                if let Ok(compensated) = apply_kerf_compensation(
                    &c.points,
                    c.closed,
                    c.is_inner_hole,
                    config.kerf,
                    config.kerf_mode,
                ) {
                    *c = CamContour::new(
                        compensated,
                        c.closed,
                        c.shape_id,
                        c.layer_name.clone(),
                        c.mode,
                    );
                }
            }
        }
        // Recalculate containment after kerf offset
        calculate_containment_depths(&mut raw_contours);
    }

    // 4. Order toolpaths with containment & travel optimization
    let start_pt = Point2::new(config.start_x, config.start_y);
    let ordered_contours = optimize_cutting_order(
        raw_contours,
        start_pt,
        config.inside_out,
        config.optimize_order,
    );

    // 5. Assemble discrete toolpaths and rapid travel vectors
    let mut toolpaths = Vec::with_capacity(ordered_contours.len());
    let mut rapid_moves = Vec::new();
    let mut total_cut_len = 0.0;
    let mut total_rapid_len = 0.0;
    let mut cut_duration = 0.0;
    let mut rapid_duration = 0.0;
    let mut pierce_count = 0;

    let mut current_pos = start_pt;

    for (seq_idx, c) in ordered_contours.into_iter().enumerate() {
        let settings = config
            .shape_overrides
            .get(&c.shape_id)
            .cloned()
            .or_else(|| config.layer_settings.get(&c.layer_name).cloned())
            .unwrap_or_else(|| LayerLaserSettings::new_cut(25.0, 80.0, 0));

        let cut_len = c.length_mm();
        total_cut_len += cut_len;

        // Rapid travel from current position to contour start
        let start_cut = c.points[0];
        let rapid_dist = current_pos.distance(start_cut);
        if rapid_dist > 0.01 {
            rapid_moves.push((current_pos, start_cut));
            total_rapid_len += rapid_dist;
            rapid_duration += trapezoidal_move_time(
                rapid_dist,
                config.rapid_speed_mm_s,
                config.rapid_acceleration_mm_s2,
            );
        }

        // Cut duration with trapezoidal acceleration profile per pass
        let path_cut_time = if settings.speed_mm_s > 0.0 {
            let mut seg_time = 0.0;
            for i in 1..c.points.len() {
                let d = c.points[i - 1].distance(c.points[i]);
                seg_time +=
                    trapezoidal_move_time(d, settings.speed_mm_s, config.acceleration_mm_s2);
            }
            if c.closed && c.points.len() >= 3 {
                let d = c.points.last().unwrap().distance(c.points[0]);
                seg_time +=
                    trapezoidal_move_time(d, settings.speed_mm_s, config.acceleration_mm_s2);
            }
            seg_time
        } else {
            0.0
        };

        let passes = settings.passes.max(1) as f64;
        let pass_pierces = passes as usize;
        pierce_count += pass_pierces;

        cut_duration += (path_cut_time + config.pierce_delay_s) * passes;

        // Update current position to end of cut
        current_pos = if c.closed {
            c.points[0]
        } else {
            *c.points.last().unwrap()
        };

        toolpaths.push(Toolpath {
            shape_id: c.shape_id,
            layer_name: c.layer_name,
            mode: c.mode,
            points: c.points,
            closed: c.closed,
            speed_mm_s: settings.speed_mm_s,
            power_pct: settings.power_pct,
            passes: settings.passes,
            air_assist: settings.air_assist,
            cut_length_mm: cut_len,
            sequence_index: seq_idx + 1,
        });
    }

    let total_duration = cut_duration + rapid_duration;
    let estimation = CamJobEstimation {
        total_cut_length_mm: total_cut_len,
        total_rapid_length_mm: total_rapid_len,
        total_raster_length_mm: 0.0,
        total_distance_mm: total_cut_len + total_rapid_len,
        cut_duration_s: cut_duration,
        rapid_duration_s: rapid_duration,
        raster_duration_s: 0.0,
        total_duration_s: total_duration,
        pierce_count,
        is_controller_verified: false,
    };

    CamJobPlan {
        toolpaths,
        rapid_moves,
        total_cut_length_mm: total_cut_len,
        total_rapid_length_mm: total_rapid_len,
        estimated_duration_s: total_duration,
        estimation,
    }
}
