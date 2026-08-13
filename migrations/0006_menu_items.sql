-- 酒单 CRUD（替代 get_menu 硬编码）。menu_items 表 + seed 搬现有硬编码酒单。
-- visible=0 隐藏项（admin 可见；前端只读看单，不下单，见 design 定稿）。
CREATE TABLE IF NOT EXISTS menu_items (
    id          TEXT PRIMARY KEY,
    section     TEXT NOT NULL,
    name        TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    price       INTEGER NOT NULL DEFAULT 0,
    sort_order  INTEGER NOT NULL DEFAULT 0,
    visible     INTEGER NOT NULL DEFAULT 1,
    created_at  INTEGER NOT NULL
);

-- seed: 搬 get_menu 硬编码酒单（3 section，8 项）。visible=1，按 section+sort_order 排序。
INSERT OR IGNORE INTO menu_items (id, section, name, description, price, sort_order, visible, created_at) VALUES
('seed-old-fashioned', 'Signature Cocktails', 'Imbibe Old Fashioned', 'Bourbon, bitters, a whisper of smoke', 12, 0, 1, 0),
('seed-pixel-sour',    'Signature Cocktails', 'Pixel Sour', 'Gin, lemon, egg white, 8-bit cherry', 11, 1, 1, 0),
('seed-mosaic-mule',   'Signature Cocktails', 'Mosaic Mule', 'Vodka, ginger beer, lime, copper mug', 10, 2, 1, 0),
('seed-negroni',          'Classics', 'Negroni', 'Gin, Campari, sweet vermouth', 11, 0, 1, 0),
('seed-margarita',        'Classics', 'Margarita', 'Tequila, lime, triple sec, salt rim', 10, 1, 1, 0),
('seed-espresso-martini', 'Classics', 'Espresso Martini', 'Vodka, coffee liqueur, fresh espresso', 12, 2, 1, 0),
('seed-garden-spritz', 'Zero Proof', 'Garden Spritz', 'Cucumber, mint, soda', 7, 0, 1, 0),
('seed-berry-fizz',    'Zero Proof', 'Berry Fizz', 'Mixed berries, lemon, tonic', 7, 1, 1, 0);
