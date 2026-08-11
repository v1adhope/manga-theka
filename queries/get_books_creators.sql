select bc.book_id, c.id, c.first_name, c.last_name, c.role, c.created_at
from creators c
join book_creators bc on bc.creator_id = c.id
where bc.book_id = any($1::uuid[])
order by c.id;
