update chapter_releases
set version = version + 1
where id = $1 and version = $2
returning version;
