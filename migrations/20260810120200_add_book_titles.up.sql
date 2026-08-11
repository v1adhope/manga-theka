create table if not exists book_titles (
	id uuid,
	book_id uuid not null,
	language_id uuid not null,
	name text not null,

	constraint pk_book_titles_id primary key(id),
	constraint fk_book_titles_books_book_id foreign key(book_id) references books(id) on delete cascade,
	constraint fk_book_titles_languages_language_id foreign key(language_id) references languages(id) on delete restrict,
	constraint check_length_book_titles_name check(char_length(name) <= 255),
	constraint unique_book_titles_book_id_language_id_name unique(book_id, language_id, name)
);

-- Backstops the 12-per-book cap that request validation enforces first. In
-- normal operation this never fires; it exists for write paths that skip the
-- application layer.
create or replace function enforce_book_titles_limit() returns trigger as $$
begin
	if (select count(*) from book_titles where book_id = new.book_id) > 12 then
		raise exception 'book % exceeds the alternative title limit of 12', new.book_id
			using errcode = 'check_violation',
				  constraint = 'check_count_book_titles_book_id';
	end if;

	return null;
end;
$$ language plpgsql;

create or replace trigger enforce_book_titles_limit
after insert on book_titles
for each row
execute function enforce_book_titles_limit();
