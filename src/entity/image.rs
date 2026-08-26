use bytes::Bytes;
use uuid::Uuid;

use crate::{
    entity::{DEFAULT_IMAGE_MAX_BYTES, ImageExtension},
    error::EntityError,
};

const MAX_FILE_NAME_CHARS: usize = 255;

#[derive(Debug, Clone)]
pub struct ImageContent(Bytes);

impl TryFrom<Bytes> for ImageContent {
    type Error = EntityError;

    fn try_from(bytes: Bytes) -> Result<Self, Self::Error> {
        if bytes.len() > DEFAULT_IMAGE_MAX_BYTES {
            return Err(EntityError::ImageExceedsByteLimit(DEFAULT_IMAGE_MAX_BYTES));
        }

        Ok(Self(bytes))
    }
}

impl AsRef<[u8]> for ImageContent {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl ImageContent {
    pub fn to_bytes(&self) -> Bytes {
        self.0.clone()
    }
}

#[derive(Debug)]
pub struct Image {
    pub id: Uuid,
    pub extension: ImageExtension,
    pub content: ImageContent,
    pub file_name: Option<String>,
}

impl Image {
    pub fn new(
        id: Uuid,
        content: ImageContent,
        file_name: Option<String>,
    ) -> Result<Self, EntityError> {
        let extension = ImageExtension::try_from(content.as_ref())?;

        Ok(Self {
            id,
            extension,
            content,
            file_name,
        })
    }

    pub fn content_disposition(&self) -> String {
        match self.sanitized_file_name() {
            Some(name) => format!("inline; filename=\"{name}\""),
            None => self.extension.content_disposition(self.id),
        }
    }

    fn sanitized_file_name(&self) -> Option<String> {
        let name = self.file_name.as_deref()?;

        let sanitized: String = name
            .chars()
            .filter(|c| !c.is_control() && *c != '"' && *c != '\\')
            .take(MAX_FILE_NAME_CHARS)
            .collect();

        if sanitized.is_empty() {
            return None;
        }

        Some(sanitized)
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use uuid::Uuid;

    use crate::entity::{DEFAULT_IMAGE_MAX_BYTES, Image, ImageContent, ImageExtension};

    const PNG_SIGNATURE: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

    #[test]
    fn content_at_the_byte_ceiling_is_valid() {
        let bytes = Bytes::from(vec![0u8; DEFAULT_IMAGE_MAX_BYTES]);
        assert!(ImageContent::try_from(bytes).is_ok());
    }

    #[test]
    fn content_over_the_byte_ceiling_is_rejected() {
        let bytes = Bytes::from(vec![0u8; DEFAULT_IMAGE_MAX_BYTES + 1]);
        assert!(ImageContent::try_from(bytes).is_err());
    }

    #[test]
    fn content_disposition_falls_back_to_the_id_suffixed_filename_without_a_client_name() {
        let content = ImageContent::try_from(Bytes::from(PNG_SIGNATURE.to_vec())).unwrap();
        let image = Image::new(Uuid::from_u128(1), content, None).unwrap();

        assert_eq!(
            image.content_disposition(),
            "inline; filename=\"00000000-0000-0000-0000-000000000001.png\""
        );
    }

    #[test]
    fn content_disposition_uses_the_client_file_name_when_present() {
        let content = ImageContent::try_from(Bytes::from(PNG_SIGNATURE.to_vec())).unwrap();
        let image = Image::new(
            Uuid::from_u128(1),
            content,
            Some("cover art.png".to_owned()),
        )
        .unwrap();

        assert_eq!(
            image.content_disposition(),
            "inline; filename=\"cover art.png\""
        );
    }

    #[test]
    fn content_disposition_strips_quotes_backslashes_and_control_characters() {
        let content = ImageContent::try_from(Bytes::from(PNG_SIGNATURE.to_vec())).unwrap();
        let image = Image::new(
            Uuid::from_u128(1),
            content,
            Some("evil\"\\\r\nname.png".to_owned()),
        )
        .unwrap();

        assert_eq!(
            image.content_disposition(),
            "inline; filename=\"evilname.png\""
        );
    }

    #[test]
    fn content_disposition_falls_back_when_the_client_file_name_sanitizes_to_empty() {
        let content = ImageContent::try_from(Bytes::from(PNG_SIGNATURE.to_vec())).unwrap();
        let image = Image::new(Uuid::from_u128(1), content, Some("\"\\".to_owned())).unwrap();

        assert_eq!(
            image.content_disposition(),
            "inline; filename=\"00000000-0000-0000-0000-000000000001.png\""
        );
    }

    #[test]
    fn content_disposition_truncates_a_client_file_name_over_the_char_limit() {
        let content = ImageContent::try_from(Bytes::from(PNG_SIGNATURE.to_vec())).unwrap();
        let long_name = "a".repeat(300);
        let image = Image::new(Uuid::from_u128(1), content, Some(long_name)).unwrap();

        let expected = format!("inline; filename=\"{}\"", "a".repeat(255));
        assert_eq!(image.content_disposition(), expected);
    }

    #[test]
    fn new_sniffs_the_extension_from_content_rather_than_trusting_the_caller() {
        let content = ImageContent::try_from(Bytes::from(PNG_SIGNATURE.to_vec())).unwrap();
        let image = Image::new(Uuid::from_u128(1), content, None).unwrap();

        assert_eq!(image.extension, ImageExtension::Png);
    }

    #[test]
    fn new_rejects_unsupported_formats() {
        let content = ImageContent::try_from(Bytes::from_static(b"GIF89a")).unwrap();
        let res = Image::new(Uuid::from_u128(1), content, None);

        assert!(res.is_err());
    }
}
