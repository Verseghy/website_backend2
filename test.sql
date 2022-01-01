SELECT *
FROM (`canteen_menus`
	INNER JOIN `canteen_menus`
	ON `canteen_pivot_menus_data`.`menu_id` = `canteen_menus`.`id`);
