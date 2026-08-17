select bc.id as "id?", bc.extension as "extension?", bc.is_main as "is_main?"
from books b
left join book_covers bc on bc.book_id = b.id
where b.id = $1
order by bc.id;
