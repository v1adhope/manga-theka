select bc.book_id, c.id, c.first_name, c.last_name,
       array_agg(bc.role order by bc.role) as "roles!", c.created_at
from book_creators bc
join creators c on c.id = bc.creator_id
where bc.book_id = any($1::uuid[])
group by bc.book_id, c.id
order by c.id;
