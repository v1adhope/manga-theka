update chapters
set number = $2, name = $3, volume = $4, updated_at = $5
where id = $1
returning id;
