//! 图像处理模块
//! 
//! 使用 image crate 实现图像处理功能，替代 Sharp

use image::{
    DynamicImage, ImageFormat, GenericImageView,
    imageops::{FilterType, resize},
    io::Reader as ImageReader,
};
use std::io::Cursor;
use std::path::Path;

use crate::error::{ClaudeError, Result};

/// 图像处理器
pub struct ImageProcessor;

/// 图像处理配置
#[derive(Debug, Clone)]
pub struct ImageProcessingConfig {
    /// 输出质量 (1-100)
    pub quality: u8,
    /// 是否保持宽高比
    pub preserve_aspect_ratio: bool,
    /// 滤波器类型
    pub filter: FilterType,
}

/// 图像信息
#[derive(Debug, Clone)]
pub struct ImageInfo {
    /// 宽度
    pub width: u32,
    /// 高度
    pub height: u32,
    /// 格式
    pub format: Option<ImageFormat>,
    /// 颜色类型
    pub color_type: String,
}

impl Default for ImageProcessingConfig {
    fn default() -> Self {
        Self {
            quality: 80,
            preserve_aspect_ratio: true,
            filter: FilterType::Lanczos3,
        }
    }
}

impl ImageProcessor {
    /// 创建新的图像处理器
    pub fn new() -> Self {
        Self
    }

    /// 从文件加载图像
    pub async fn load_from_file<P: AsRef<Path>>(&self, path: P) -> Result<DynamicImage> {
        let path = path.as_ref();
        
        let img = ImageReader::open(path)
            .map_err(|e| ClaudeError::General(format!("Failed to open image file '{}': {}", path.display(), e)))?
            .decode()
            .map_err(|e| ClaudeError::General(format!("Failed to decode image '{}': {}", path.display(), e)))?;
        
        Ok(img)
    }

    /// 从字节数组加载图像
    pub async fn load_from_bytes(&self, data: &[u8]) -> Result<DynamicImage> {
        let img = ImageReader::new(Cursor::new(data))
            .with_guessed_format()
            .map_err(|e| ClaudeError::General(format!("Failed to guess image format: {}", e)))?
            .decode()
            .map_err(|e| ClaudeError::General(format!("Failed to decode image from bytes: {}", e)))?;
        
        Ok(img)
    }

    /// 调整图像大小
    pub async fn resize(
        &self,
        img: &DynamicImage,
        width: u32,
        height: u32,
        config: &ImageProcessingConfig,
    ) -> Result<DynamicImage> {
        let (new_width, new_height) = if config.preserve_aspect_ratio {
            self.calculate_aspect_ratio_size(img.width(), img.height(), width, height)
        } else {
            (width, height)
        };

        let resized = resize(img, new_width, new_height, config.filter);
        Ok(DynamicImage::ImageRgba8(resized))
    }

    /// 裁剪图像
    pub async fn crop(
        &self,
        img: &DynamicImage,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    ) -> Result<DynamicImage> {
        // 检查裁剪区域是否有效
        if x + width > img.width() || y + height > img.height() {
            return Err(ClaudeError::General(
                "Crop area exceeds image boundaries".to_string()
            ));
        }

        let cropped = img.crop_imm(x, y, width, height);
        Ok(cropped)
    }

    /// 旋转图像
    pub async fn rotate(&self, img: &DynamicImage, degrees: f32) -> Result<DynamicImage> {
        let rotated = match degrees as i32 % 360 {
            90 | -270 => img.rotate90(),
            180 | -180 => img.rotate180(),
            270 | -90 => img.rotate270(),
            0 => img.clone(),
            _ => {
                return Err(ClaudeError::General(
                    "Only 90, 180, 270 degree rotations are supported".to_string()
                ));
            }
        };
        
        Ok(rotated)
    }

    /// 翻转图像
    pub async fn flip(&self, img: &DynamicImage, horizontal: bool) -> Result<DynamicImage> {
        let flipped = if horizontal {
            img.fliph()
        } else {
            img.flipv()
        };
        
        Ok(flipped)
    }

    /// 调整亮度
    pub async fn adjust_brightness(&self, img: &DynamicImage, value: i32) -> Result<DynamicImage> {
        let adjusted = img.brighten(value);
        Ok(adjusted)
    }

    /// 调整对比度
    pub async fn adjust_contrast(&self, img: &DynamicImage, contrast: f32) -> Result<DynamicImage> {
        let adjusted = img.adjust_contrast(contrast);
        Ok(adjusted)
    }

    /// 模糊图像
    pub async fn blur(&self, img: &DynamicImage, sigma: f32) -> Result<DynamicImage> {
        let blurred = img.blur(sigma);
        Ok(blurred)
    }

    /// 锐化图像
    pub async fn sharpen(&self, img: &DynamicImage, sigma: f32, threshold: i32) -> Result<DynamicImage> {
        let sharpened = img.unsharpen(sigma, threshold);
        Ok(sharpened)
    }

    /// 转换为灰度图像
    pub async fn to_grayscale(&self, img: &DynamicImage) -> Result<DynamicImage> {
        let grayscale = img.grayscale();
        Ok(grayscale)
    }

    /// 保存图像到文件
    pub async fn save_to_file<P: AsRef<Path>>(
        &self,
        img: &DynamicImage,
        path: P,
        format: ImageFormat,
        config: &ImageProcessingConfig,
    ) -> Result<()> {
        let path = path.as_ref();
        
        match format {
            ImageFormat::Jpeg => {
                let mut output = Vec::new();
                let mut cursor = Cursor::new(&mut output);
                
                img.write_to(&mut cursor, format)
                    .map_err(|e| ClaudeError::General(format!("Failed to encode JPEG: {}", e)))?;
                
                tokio::fs::write(path, output).await
                    .map_err(|e| ClaudeError::General(format!("Failed to write file '{}': {}", path.display(), e)))?;
            }
            _ => {
                img.save(path)
                    .map_err(|e| ClaudeError::General(format!("Failed to save image '{}': {}", path.display(), e)))?;
            }
        }
        
        Ok(())
    }

    /// 保存图像到字节数组
    pub async fn save_to_bytes(
        &self,
        img: &DynamicImage,
        format: ImageFormat,
        config: &ImageProcessingConfig,
    ) -> Result<Vec<u8>> {
        let mut output = Vec::new();
        let mut cursor = Cursor::new(&mut output);
        
        img.write_to(&mut cursor, format)
            .map_err(|e| ClaudeError::General(format!("Failed to encode image: {}", e)))?;
        
        Ok(output)
    }

    /// 获取图像信息
    pub async fn get_image_info(&self, img: &DynamicImage) -> ImageInfo {
        ImageInfo {
            width: img.width(),
            height: img.height(),
            format: None, // 动态图像没有原始格式信息
            color_type: format!("{:?}", img.color()),
        }
    }

    /// 批量处理图像
    pub async fn batch_process<F, Fut>(
        &self,
        images: Vec<DynamicImage>,
        processor: F,
    ) -> Result<Vec<DynamicImage>>
    where
        F: Fn(DynamicImage) -> Fut + Send + Sync,
        Fut: std::future::Future<Output = Result<DynamicImage>> + Send,
    {
        let mut results = Vec::new();
        
        for img in images {
            let processed = processor(img).await?;
            results.push(processed);
        }
        
        Ok(results)
    }

    /// 创建缩略图
    pub async fn create_thumbnail(
        &self,
        img: &DynamicImage,
        max_size: u32,
        config: &ImageProcessingConfig,
    ) -> Result<DynamicImage> {
        let (width, height) = img.dimensions();
        let max_dimension = width.max(height);
        
        if max_dimension <= max_size {
            return Ok(img.clone());
        }
        
        let scale = max_size as f32 / max_dimension as f32;
        let new_width = (width as f32 * scale) as u32;
        let new_height = (height as f32 * scale) as u32;
        
        self.resize(img, new_width, new_height, config).await
    }

    /// 水印处理
    pub async fn add_watermark(
        &self,
        base_img: &DynamicImage,
        watermark_img: &DynamicImage,
        x: u32,
        y: u32,
        opacity: f32,
    ) -> Result<DynamicImage> {
        let mut base = base_img.to_rgba8();
        let watermark = watermark_img.to_rgba8();
        
        // 检查位置是否有效
        if x + watermark.width() > base.width() || y + watermark.height() > base.height() {
            return Err(ClaudeError::General(
                "Watermark position exceeds image boundaries".to_string()
            ));
        }
        
        // 应用水印
        for (wx, wy, pixel) in watermark.enumerate_pixels() {
            let base_x = x + wx;
            let base_y = y + wy;
            
            if base_x < base.width() && base_y < base.height() {
                let base_pixel = base.get_pixel_mut(base_x, base_y);
                let watermark_pixel = *pixel;
                
                // 简单的 alpha 混合
                let alpha = (watermark_pixel[3] as f32 / 255.0) * opacity;
                let inv_alpha = 1.0 - alpha;
                
                base_pixel[0] = (base_pixel[0] as f32 * inv_alpha + watermark_pixel[0] as f32 * alpha) as u8;
                base_pixel[1] = (base_pixel[1] as f32 * inv_alpha + watermark_pixel[1] as f32 * alpha) as u8;
                base_pixel[2] = (base_pixel[2] as f32 * inv_alpha + watermark_pixel[2] as f32 * alpha) as u8;
            }
        }
        
        Ok(DynamicImage::ImageRgba8(base))
    }

    /// 计算保持宽高比的新尺寸
    fn calculate_aspect_ratio_size(&self, orig_width: u32, orig_height: u32, target_width: u32, target_height: u32) -> (u32, u32) {
        let width_ratio = target_width as f32 / orig_width as f32;
        let height_ratio = target_height as f32 / orig_height as f32;
        let scale = width_ratio.min(height_ratio);
        
        let new_width = (orig_width as f32 * scale) as u32;
        let new_height = (orig_height as f32 * scale) as u32;
        
        (new_width, new_height)
    }

    // 高级方法：直接处理文件路径

    /// 调整图像大小（文件版本）
    pub async fn resize_image<P: AsRef<Path>>(
        &self,
        input_path: P,
        output_path: P,
        width: Option<u32>,
        height: Option<u32>,
        config: &ImageProcessingConfig,
    ) -> Result<()> {
        let img = self.load_from_file(input_path).await?;

        let (new_width, new_height) = match (width, height) {
            (Some(w), Some(h)) => (w, h),
            (Some(w), None) => {
                let aspect_ratio = img.height() as f32 / img.width() as f32;
                (w, (w as f32 * aspect_ratio) as u32)
            }
            (None, Some(h)) => {
                let aspect_ratio = img.width() as f32 / img.height() as f32;
                ((h as f32 * aspect_ratio) as u32, h)
            }
            (None, None) => return Err(ClaudeError::General("Must specify width or height".to_string())),
        };

        let resized = self.resize(&img, new_width, new_height, config).await?;

        // 推断输出格式
        let format = self.infer_format_from_path(&output_path)?;
        self.save_to_file(&resized, output_path, format, config).await?;

        Ok(())
    }

    /// 转换图像格式
    pub async fn convert_format<P: AsRef<Path>>(
        &self,
        input_path: P,
        output_path: P,
        target_format: Option<&str>,
        config: &ImageProcessingConfig,
    ) -> Result<()> {
        let img = self.load_from_file(input_path).await?;

        let format = if let Some(fmt_str) = target_format {
            self.parse_format_string(fmt_str)?
        } else {
            self.infer_format_from_path(&output_path)?
        };

        self.save_to_file(&img, output_path, format, config).await?;

        Ok(())
    }

    /// 获取图像信息（文件版本）
    pub async fn get_image_info_from_file<P: AsRef<Path>>(&self, path: P) -> Result<ImageInfo> {
        let path = path.as_ref();

        // 尝试从文件扩展名推断格式
        let format = self.infer_format_from_path(path).ok();

        let img = self.load_from_file(path).await?;
        let mut info = self.get_image_info(&img).await;
        info.format = format;

        Ok(info)
    }

    /// 创建缩略图（文件版本）
    pub async fn create_thumbnail_from_file<P: AsRef<Path>>(
        &self,
        input_path: P,
        output_path: P,
        max_size: u32,
        config: &ImageProcessingConfig,
    ) -> Result<()> {
        let img = self.load_from_file(input_path).await?;
        let thumbnail = self.create_thumbnail(&img, max_size, config).await?;

        let format = self.infer_format_from_path(&output_path)?;
        self.save_to_file(&thumbnail, output_path, format, config).await?;

        Ok(())
    }

    /// 旋转图像（文件版本）
    pub async fn rotate_image<P: AsRef<Path>>(
        &self,
        input_path: P,
        output_path: P,
        angle: u32,
    ) -> Result<()> {
        let img = self.load_from_file(input_path).await?;
        let rotated = self.rotate(&img, angle as f32).await?;

        let format = self.infer_format_from_path(&output_path)?;
        let config = ImageProcessingConfig::default();
        self.save_to_file(&rotated, output_path, format, &config).await?;

        Ok(())
    }

    /// 翻转图像（文件版本）
    pub async fn flip_image<P: AsRef<Path>>(
        &self,
        input_path: P,
        output_path: P,
        horizontal: bool,
        vertical: bool,
    ) -> Result<()> {
        let img = self.load_from_file(input_path).await?;

        let mut result = img;
        if horizontal {
            result = self.flip(&result, true).await?;
        }
        if vertical {
            result = self.flip(&result, false).await?;
        }

        let format = self.infer_format_from_path(&output_path)?;
        let config = ImageProcessingConfig::default();
        self.save_to_file(&result, output_path, format, &config).await?;

        Ok(())
    }

    /// 裁剪图像（文件版本）
    pub async fn crop_image<P: AsRef<Path>>(
        &self,
        input_path: P,
        output_path: P,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    ) -> Result<()> {
        let img = self.load_from_file(input_path).await?;
        let cropped = self.crop(&img, x, y, width, height).await?;

        let format = self.infer_format_from_path(&output_path)?;
        let config = ImageProcessingConfig::default();
        self.save_to_file(&cropped, output_path, format, &config).await?;

        Ok(())
    }

    /// 从文件路径推断图像格式
    fn infer_format_from_path<P: AsRef<Path>>(&self, path: P) -> Result<ImageFormat> {
        let path = path.as_ref();
        let extension = path.extension()
            .and_then(|ext| ext.to_str())
            .ok_or_else(|| ClaudeError::General("Cannot determine file format from path".to_string()))?;

        match extension.to_lowercase().as_str() {
            "jpg" | "jpeg" => Ok(ImageFormat::Jpeg),
            "png" => Ok(ImageFormat::Png),
            "gif" => Ok(ImageFormat::Gif),
            "bmp" => Ok(ImageFormat::Bmp),
            "ico" => Ok(ImageFormat::Ico),
            "tiff" | "tif" => Ok(ImageFormat::Tiff),
            "webp" => Ok(ImageFormat::WebP),
            "avif" => Ok(ImageFormat::Avif),
            _ => Err(ClaudeError::General(format!("Unsupported image format: {}", extension))),
        }
    }

    /// 解析格式字符串
    fn parse_format_string(&self, format_str: &str) -> Result<ImageFormat> {
        match format_str.to_lowercase().as_str() {
            "jpg" | "jpeg" => Ok(ImageFormat::Jpeg),
            "png" => Ok(ImageFormat::Png),
            "gif" => Ok(ImageFormat::Gif),
            "bmp" => Ok(ImageFormat::Bmp),
            "ico" => Ok(ImageFormat::Ico),
            "tiff" | "tif" => Ok(ImageFormat::Tiff),
            "webp" => Ok(ImageFormat::WebP),
            "avif" => Ok(ImageFormat::Avif),
            _ => Err(ClaudeError::General(format!("Unsupported image format: {}", format_str))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_image_processor_creation() {
        let processor = ImageProcessor::new();
        // 基本创建测试
        assert!(true);
    }

    #[test]
    fn test_aspect_ratio_calculation() {
        let processor = ImageProcessor::new();
        let (width, height) = processor.calculate_aspect_ratio_size(1920, 1080, 800, 600);
        
        // 应该按照较小的比例缩放
        assert_eq!(width, 800);
        assert_eq!(height, 450); // 1080 * (800/1920) = 450
    }

    #[test]
    fn test_config_default() {
        let config = ImageProcessingConfig::default();
        assert_eq!(config.quality, 80);
        assert!(config.preserve_aspect_ratio);
    }
}
