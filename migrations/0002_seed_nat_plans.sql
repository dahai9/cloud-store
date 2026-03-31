INSERT
    OR IGNORE INTO nat_plans (
        id,
        code,
        name,
        memory_mb,
        storage_gb,
        monthly_price,
        active
    )
VALUES (
        '11111111-1111-1111-1111-111111111111',
        'nat-mini',
        'NAT Mini',
        1024,
        50,
        5.99,
        1
    ),
    (
        '22222222-2222-2222-2222-222222222222',
        'nat-standard',
        'NAT Standard',
        1024,
        50,
        7.99,
        1
    ),
    (
        '33333333-3333-3333-3333-333333333333',
        'nat-pro',
        'NAT Pro',
        1024,
        50,
        9.99,
        1
    );