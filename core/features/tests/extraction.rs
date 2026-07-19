//! Feature extraction from real DSP window results.

use std::sync::Arc;

use aeryon_calibration::{CalibrationPipeline, baseline_csi_v1};
use aeryon_csi::{ComplexSample, CsiFrame, CsiRadioMetadata, CsiSourceKind};
use aeryon_domain::{FrameId, FrameMetadata, Metadata, SensorId, Timestamp};
use aeryon_dsp::assembler::AssemblerConfig;
use aeryon_dsp::{DspBackendKind, baseline_dsp_v1, process_window};
use aeryon_dsp::{RustKernelBackend, WindowAssembler};
use aeryon_features::{FeatureId, baseline_features_v1, csi_channel_features_v1, extract_features};

fn calibrated(sequence: u64) -> Arc<aeryon_calibration::CalibratedCsiFrame> {
    let phase = sequence as f32 * 0.15;
    let samples = vec![
        ComplexSample::new(phase.cos(), phase.sin()),
        ComplexSample::new((phase * 1.3).cos(), (phase * 1.3).sin()),
        ComplexSample::new(0.8 * phase.cos(), 0.8 * phase.sin()),
        ComplexSample::new(1.1 * (phase * 0.7).cos(), 1.1 * (phase * 0.7).sin()),
    ];
    let nanos = sequence * 100_000_000;
    let metadata = FrameMetadata {
        frame_id: FrameId::new(sequence + 1),
        sensor_id: SensorId::new(2),
        timestamp: Timestamp::from_nanos(nanos),
        sequence,
        mission_id: None,
        metadata: Metadata::new(),
    };
    let raw = CsiFrame::try_new(
        metadata,
        Timestamp::from_nanos(nanos),
        None,
        None,
        2,
        1,
        vec![0, 1],
        samples,
        CsiSourceKind::Replay,
        CsiRadioMetadata::default(),
    )
    .expect("raw");
    let pipeline = CalibrationPipeline::try_new(baseline_csi_v1()).expect("pipeline");
    Arc::new(pipeline.calibrate(Arc::new(raw)).expect("calibrated"))
}

#[test]
fn extract_ordered_finite_vector_with_provenance() {
    let profile = baseline_dsp_v1(8, 4, 0.10, DspBackendKind::Rust);
    let mut assembler = WindowAssembler::try_new(AssemblerConfig {
        window_size_frames: 8,
        hop_size_frames: 4,
        queue_capacity: 16,
        maximum_sequence_gap: 1,
        timestamp_jitter_tolerance: 0.10,
    })
    .expect("assembler");

    let mut window = None;
    for sequence in 0..8 {
        if let Some(assembled) = assembler.push(calibrated(sequence)).expect("push") {
            window = Some(assembled);
        }
    }
    let window = window.expect("window");
    let dsp = process_window(&window, &profile, &RustKernelBackend).expect("dsp");
    let original_motion = dsp.motion_energy.signal.aggregate.clone();

    let feature_profile = baseline_features_v1();
    let (vector, report) = extract_features(&dsp, &feature_profile).expect("features");
    let schema = csi_channel_features_v1();

    assert_eq!(vector.feature_schema_id, schema.id);
    assert_eq!(vector.feature_count(), schema.length());
    assert_eq!(vector.values().len(), FeatureId::ALL.len());
    assert!(vector.values().iter().all(|value| value.is_finite()));
    assert_eq!(report.features_produced, schema.length());
    assert_eq!(vector.calibration_profile_id, "baseline-csi-v1");
    assert_eq!(vector.dsp_profile_id, "baseline-dsp-v1");
    assert_eq!(vector.dsp_backend_id, "rust");

    let rms_index = schema.index_of(FeatureId::MotionEnergyRms).unwrap();
    assert!(vector.value_at(rms_index).unwrap().is_finite());

    let (again, _) = extract_features(&dsp, &feature_profile).expect("repeat");
    assert_eq!(vector.values(), again.values());
    assert_eq!(dsp.motion_energy.signal.aggregate, original_motion);
}

#[test]
fn incompatible_dsp_profile_rejected() {
    let profile = baseline_dsp_v1(8, 4, 0.10, DspBackendKind::Rust);
    let mut assembler = WindowAssembler::try_new(AssemblerConfig {
        window_size_frames: 8,
        hop_size_frames: 4,
        queue_capacity: 16,
        maximum_sequence_gap: 1,
        timestamp_jitter_tolerance: 0.10,
    })
    .expect("assembler");
    let mut window = None;
    for sequence in 0..8 {
        if let Some(assembled) = assembler.push(calibrated(sequence)).expect("push") {
            window = Some(assembled);
        }
    }
    let dsp = process_window(&window.expect("window"), &profile, &RustKernelBackend).expect("dsp");
    let mut mismatched = dsp.clone();
    mismatched.dsp_profile_id = "not-a-profile".to_owned();
    assert!(extract_features(&mismatched, &baseline_features_v1()).is_err());
}
