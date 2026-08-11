select creators.id, creators.first_name, creators.last_name, creators.role, creators.created_at
from creators
join book_creators on book_creators.creator_id = creators.id
where book_creators.book_id = $1
order by creators.last_name, creators.first_name;
