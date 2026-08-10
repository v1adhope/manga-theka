select labels.id, labels.name, labels.kind
from labels
join book_labels on book_labels.label_id = labels.id
where book_labels.book_id = $1
order by labels.name;
