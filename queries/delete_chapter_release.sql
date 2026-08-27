delete from chapter_releases
where id = $1
returning id;
