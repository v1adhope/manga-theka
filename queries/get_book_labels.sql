select l.id, l.name, l.kind
from labels l
join book_labels bl on bl.label_id = l.id
where bl.book_id = $1
order by l.name;
