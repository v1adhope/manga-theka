use serde::{Deserialize, Serialize};

use crate::entity::{CoverUrl, PageUrl};

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct ResourceUrl(String);

impl From<CoverUrl> for ResourceUrl {
    fn from(url: CoverUrl) -> Self {
        Self(format!("/covers/{}/image", url.cover_id))
    }
}

impl From<PageUrl> for ResourceUrl {
    fn from(url: PageUrl) -> Self {
        Self(format!(
            "/releases/{}/pages/{}",
            url.release_id,
            url.page_number.as_i32()
        ))
    }
}

impl AsRef<str> for ResourceUrl {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::entity::{CoverUrl, Ordinal, PageUrl, ResourceUrl};

    #[test]
    fn cover_url_points_at_the_cover_scoped_image_route() {
        let url = ResourceUrl::from(CoverUrl {
            cover_id: Uuid::from_u128(1),
        });

        assert_eq!(
            url.as_ref(),
            "/covers/00000000-0000-0000-0000-000000000001/image"
        );
    }

    #[test]
    fn page_url_points_at_the_position_addressed_reader_route() {
        let url = ResourceUrl::from(PageUrl {
            release_id: Uuid::from_u128(1),
            page_number: Ordinal::try_from(4).unwrap(),
        });

        assert_eq!(
            url.as_ref(),
            "/releases/00000000-0000-0000-0000-000000000001/pages/4"
        );
    }
}
