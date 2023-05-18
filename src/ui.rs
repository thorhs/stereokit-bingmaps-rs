use glam::{Mat4, Vec2};
use stereokit_sys::*;

use std::marker::PhantomData;

pub struct Input(PhantomData<*const ()>);

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum Handed {
    Left = 0,
    Rigth = 1,
}

impl From<handed_> for Handed {
    fn from(value: handed_) -> Self {
        unsafe { std::mem::transmute(value) }
    }
}
impl Into<handed_> for Handed {
    fn into(self) -> handed_ {
        unsafe { std::mem::transmute(self) }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
#[allow(unused)]
pub enum FingerId {
    Thumb = 0,
    Index = 1,
    Middle = 2,
    Ring = 3,
    Little = 4,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
#[allow(unused)]
pub enum JointId {
    Root = 0,
    KnuckleMajor = 1,
    KnuckleMid = 2,
    KnuckleMinor = 3,
    Tip = 4,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum UIConfirm {
    Push = stereokit_sys::ui_confirm__ui_confirm_push,
    Pinch = stereokit_sys::ui_confirm__ui_confirm_pinch,
    VariablePinch = stereokit_sys::ui_confirm__ui_confirm_variable_pinch,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum UINotify {
    Change = stereokit_sys::ui_notify__ui_notify_change,
    Finalize = stereokit_sys::ui_notify__ui_notify_finalize,
}

pub fn radio(text: &str, active: bool, size: Vec2) -> bool {
    let mut is_active: bool32_t = active.into();

    unsafe {
        let label = std::ffi::CString::new(text).unwrap();
        is_active =
            stereokit_sys::ui_toggle_sz(label.as_ptr() as *const i8, &mut is_active, size.into());
    };

    is_active != 0
}

pub fn h_slider(
    text: &str,
    value: &mut f32,
    min: f32,
    max: f32,
    step: f32,
    width: f32,
    confirm_method: UIConfirm,
) -> bool {
    let label = std::ffi::CString::new(text).unwrap();

    unsafe {
        stereokit_sys::ui_hslider(
            label.as_ptr() as *const i8,
            value,
            min,
            max,
            step,
            width,
            confirm_method as ui_confirm_,
            UINotify::Change as ui_notify_,
        ) != 0
    }
}

use glam::{Quat, Vec3};
use stereokit::{Bounds, Pose};

use crate::VEC3_UP;
pub fn ui_handle_begin(id: &str, terrain_pose: &mut Pose, terrain_bounds: Bounds) {
    unsafe {
        let mut pose: stereokit_sys::pose_t = (*terrain_pose).into();
        stereokit_sys::ui_handle_begin(
            id.as_ptr() as *const i8,
            &mut pose as *mut stereokit_sys::pose_t,
            std::mem::transmute(terrain_bounds),
            false.into(),
            stereokit_sys::ui_move__ui_move_pos_only,
        );
        *terrain_pose = pose.into();
    };
}

pub fn ui_window_begin(id: &str, ui_pose: &mut Pose, size: Vec2) {
    let mut ui_pose_t: stereokit_sys::pose_t = (*ui_pose).into();
    unsafe {
        stereokit_sys::ui_window_begin(
            id.as_ptr() as *const i8,
            &mut ui_pose_t as *mut stereokit_sys::pose_t,
            size.into(),
            stereokit_sys::ui_win__ui_win_empty,
            stereokit_sys::ui_move__ui_move_face_user,
        );
    }
    *ui_pose = ui_pose_t.into();
}
pub fn ui_sameline() {
    unsafe {
        stereokit_sys::ui_sameline();
    }
}

pub fn ui_window_end() {
    unsafe {
        stereokit_sys::ui_window_end();
    }
}

pub fn ui_handle_end() {
    unsafe {
        stereokit_sys::ui_handle_end();
    }
}

pub fn look_dir(ui_dir: Vec3) -> Quat {
    let lookat = unsafe {
        stereokit_sys::quat_lookat(
            &Vec3::ZERO as *const glam::Vec3 as *const stereokit_sys::vec3,
            &(ui_dir + VEC3_UP) as *const glam::Vec3 as *const stereokit_sys::vec3,
        )
    };

    Quat::from_xyzw(lookat.x, lookat.y, lookat.z, lookat.w)
}

pub fn matrix_ts(translation: Vec3, scale: f32) -> Mat4 {
    Mat4::from_scale_rotation_translation(
        Vec3::new(scale, scale, scale),
        Quat::IDENTITY,
        translation,
    )
}
