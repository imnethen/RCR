#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Zeroable, bytemuck::Pod)]
pub struct RawUniformData {
    pub c0_rays: u32,
    pub c0_spacing: f32,
    pub c0_raylength: f32,
    pub angular_scaling: u32,
    pub spatial_scaling: f32,
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
            num_cascades: config.num_cascades,
            cur_cascade: 0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct RCConfig {
    pub c0_rays: u32,
    pub c0_spacing: f32,
    pub c0_raylength: f32,
    pub angular_scaling: u32,
    pub spatial_scaling: f32,
    pub num_cascades: u32,
    /*
    TODO:

    memory_layout (posfirst / dirfirst)
    preaveraging

    ...
    */
}

impl RCConfig {
    pub fn get_num_probes_2d(&self, window_size: (u32, u32), cascade_num: u32) -> (u32, u32) {
        let fnum = {
            let c0_num = (
                window_size.0 as f32 / self.c0_spacing,
                window_size.1 as f32 / self.c0_spacing,
            );
            let scale_div = f32::powi(self.spatial_scaling, cascade_num as i32);
            (c0_num.0 / scale_div, c0_num.1 / scale_div)
        };

        (f32::ceil(fnum.0) as u32 + 1, f32::ceil(fnum.1) as u32 + 1)
    }

    pub fn get_num_probes_1d(&self, window_size: (u32, u32), cascade_num: u32) -> u32 {
        let num_2d = self.get_num_probes_2d(window_size, cascade_num);
        num_2d.0 * num_2d.1
    }

    pub fn get_max_cascade_size(&self, window_size: (u32, u32)) -> u32 {
        let mut max: u32 = 0;
        for cascade_num in 0..self.num_cascades {
            let num_rays = self.c0_rays * u32::pow(self.angular_scaling, cascade_num);
            max = max.max(num_rays * self.get_num_probes_1d(window_size, cascade_num));
        }

        max
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
            num_cascades: 6,
        }
    }
}
