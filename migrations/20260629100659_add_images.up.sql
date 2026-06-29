create table if not exists images (
	id uuid,
	entity_id uuid not null,
	entity_type text not null,
	extention text not null,
	sort_order smallint not null,

	constraint pk_images_id primary key(id),
	constraint check_length_images_entity_type check(char_length(entity_type) <= 255),
	constraint check_length_images_extention check(char_length(extention) <= 255)
);
