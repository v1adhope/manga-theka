delete from chapter_pages
where release_id = $1 and id <> all($2::uuid[])
returning id;
