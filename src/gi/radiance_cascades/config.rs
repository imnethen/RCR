#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Zeroable, bytemuck::Pod)]
pub struct RawUniformData {
    pub c0_rays: u32,
    pub c0_spacing: f32,
    pub c0_raylength: f32,
    pub angular_scaling: u32,
    pub spatial_scaling: f32,
    pub probe_layout: u32,
    pub ringing_fix: u32,
    pub num_cascades: u32,
    pub cur_cascade: u32,
}

impl From<RCConfig> for RawUniformData {
    // TODO: cur cascade mrow
    fn from(config: RCConfig) -> Self {
        RawUniformData {
            c0_rays: config.c0_rays,
            c0_spacing: config.c0_spacing,
            c0_raylength: config.c0_raylength,
            angular_scaling: config.angular_scaling,
            spatial_scaling: config.spatial_scaling,
            probe_layout: config.probe_layout as u32,
            ringing_fix: config.ringing_fix as u32,
            num_cascades: config.num_cascades,
            cur_cascade: 0,
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ProbeLayout {
    Offset = 0,
    Stacked = 1,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum RingingFix {
    Vanilla = 0,
    Bilinear = 1,
}

impl std::fmt::Display for RingingFix {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                RingingFix::Vanilla => "Vanilla",
                RingingFix::Bilinear => "Bilinear",
            }
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RCConfig {
    pub c0_rays: u32,
    pub c0_spacing: f32,
    pub c0_raylength: f32,

    pub angular_scaling: u32,
    pub spatial_scaling: f32,

    pub probe_layout: ProbeLayout,
    pub ringing_fix: RingingFix,

    pub num_cascades: u32,
}

impl RCConfig {
    pub fn get_spatial_resolution(
        &self,
        window_size: (u32, u32),
        cascade_index: u32,
    ) -> (u32, u32) {
        let probe_spacing = self.c0_spacing * f32::powi(self.spatial_scaling, cascade_index as i32);
        let float_result = (
            window_size.0 as f32 / probe_spacing,
            window_size.1 as f32 / probe_spacing,
        );
        (
            float_result.0.ceil() as u32 + 1,
            float_result.1.ceil() as u32 + 1,
        )
    }

    pub fn get_num_probes_1d(&self, window_size: (u32, u32), cascade_num: u32) -> u32 {
        let spa_res = self.get_spatial_resolution(window_size, cascade_num);
        spa_res.0 * spa_res.1
    }

    pub fn get_cascade_size(&self, window_size: (u32, u32), cascade_index: u32) -> u32 {
        let num_rays = match cascade_index {
            0 => 1,
            _ => self.c0_rays * u32::pow(self.angular_scaling, cascade_index - 1),
        };

        let res = u32::checked_mul(num_rays, self.get_num_probes_1d(window_size, cascade_index));
        res.unwrap_or(std::u32::MAX)
    }

    pub fn get_max_cascade_size(&self, window_size: (u32, u32)) -> u32 {
        (0..self.num_cascades)
            .map(|cascade_index| self.get_cascade_size(window_size, cascade_index))
            .max()
            // TODO better error handling
            .unwrap_or(0)
    }
}

impl Default for RCConfig {
    fn default() -> Self {
        RCConfig {
            c0_rays: 4,
            c0_spacing: 1.,
            c0_raylength: 1.,

            angular_scaling: 4,
            spatial_scaling: 2.,

            probe_layout: ProbeLayout::Offset,
            ringing_fix: RingingFix::Bilinear,

            num_cascades: 7,
        }
    }
}
