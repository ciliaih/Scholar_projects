#[allow(unused_imports)]
use super::gamma;
use core::mem::transmute;

#[derive(Clone, Copy, Default)]
#[repr(C)]

/// Public structure Color
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    /// Implement three public constants Color::RED, Color::GREEN and Color::BLUE
    pub const RED: Color = Color { r: 255, g: 0, b: 0 };
    pub const GREEN: Color = Color { r: 0, g: 255, b: 0 };
    pub const BLUE: Color = Color { r: 0, g: 0, b: 255 };

    /// applies the gamma::gamma_correct correction to all components of a color.
    pub fn gamma_correct(&self) -> Self {
        Color {
            r: gamma::gamma_correct(self.r),
            g: gamma::gamma_correct(self.g),
            b: gamma::gamma_correct(self.b),
        }
    }
}
/// Ensure that each component stays within the range of an u8
fn max(input: f32) -> u8 {
    if input > 255_f32 {
        return 255;
    } else {
        return input as u8;
    }
}

impl core::ops::Mul<f32> for Color {
    type Output = Color;

    fn mul(self, mult: f32) -> Self {
        Color {
            r: max(self.r as f32 * mult),
            g: max(self.g as f32 * mult),
            b: max(self.b as f32 * mult),
        }
    }
}

impl core::ops::Div<f32> for Color {
    type Output = Color;

    fn div(self, div: f32) -> Self {
        Color {
            r: max(self.r as f32 / div),
            g: max(self.g as f32 / div),
            b: max(self.b as f32 / div),
        }
    }
}
/// Tuple of struct
#[repr(transparent)]
pub struct Image([Color; 64]);

impl Image {
    /// Returns an image filled with the color given as an argument.
    pub fn new_solid(color: Color) -> Self {
        return Image([color; 64]);
    }

    /// method giving access to the content of one particular row.
    pub fn row(&self, row: usize) -> &[Color] {
        &self.0[8 * row..8 * (row + 1)]
    }

    /// Function that builds a gradient from a given color to black
    pub fn gradient(color: Color) -> Self {
        let mut grad: Image = Image::default();
        for row in 0..8 {
            for col in 0..8 {
                grad[(row, col)] = color / (1 + row * row + col) as f32
            }
        }
        grad
    }
}

impl Default for Image {
    /// Implementing the Default trait manually
    fn default() -> Self {
        Image::new_solid(Default::default())
    }
}

impl core::ops::Index<(usize, usize)> for Image {
    type Output = Color;
    /// This fonction returns the color of a pixel in a given position
    // self refers to the object Image that contains a table
    // self.0 referes to the table
    // index is a tuple
    fn index(&self, index: (usize, usize)) -> &Color {
        let uindex: usize = (8 * index.0) + index.1;
        &self.0[uindex]
    }
}

impl core::ops::IndexMut<(usize, usize)> for Image {
    /// This function returns the color located at the given position (x,y) with a mutable reference.
    /// This function is helpfull in order to easily access to a given pixel.
    fn index_mut(&mut self, index: (usize, usize)) -> &mut Color {
        let uindex: usize = (8 * index.0) + index.1;
        &mut self.0[uindex]
    }
}

impl AsRef<[u8; 192]> for Image {
    fn as_ref(&self) -> &[u8; 192] {
        unsafe { transmute::<&Image, &[u8; 192]>(self) }
    }
}

impl AsMut<[u8; 192]> for Image {
    fn as_mut(&mut self) -> &mut [u8; 192] {
        unsafe { transmute::<&mut Image, &mut [u8; 192]>(self) }
    }
}
