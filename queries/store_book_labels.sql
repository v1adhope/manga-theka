insert into book_labels(book_id, label_id)
select $1, label_id
from unnest($2::uuid[]) as label_id;
