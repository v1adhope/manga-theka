update chapter_releases
set version = version + 1
where id = $1
returning version;
