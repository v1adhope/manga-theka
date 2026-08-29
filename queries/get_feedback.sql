select f.id, f.kind, f.status, f.email, f.note, f.book_id, f.updated_at, f.created_at
from feedback f
where f.id = $1;
