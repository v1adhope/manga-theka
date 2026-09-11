select bl.book_id, l.id, l.name, l.kind
from labels l
join book_labels bl on bl.label_id = l.id
where bl.book_id = any($1::uuid[])
order by l.name;
