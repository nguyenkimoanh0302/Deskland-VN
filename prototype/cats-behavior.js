'use strict';
// Hành vi của mèo (chỉ logic, không vẽ): đi tới một điểm ngẫu nhiên, ngồi nghỉ cạnh biểu tượng hoặc lùm cây,
// ngồi liếm lông, cuộn tròn hoặc nằm nghiêng ngủ. Mèo luôn tránh đứng dưới biểu tượng vì biểu tượng Windows nằm đè lên hình nền.

const CAT_WALK_SPEED = 42;             // px/giây ở tốc độ ×1
const CAT_SIT_SECONDS = [3, 9];
const CAT_SLEEP_SECONDS = [10, 26];
const CAT_SNUGGLE_CHANCE = 0.45;       // tỉ lệ chọn chỗ ngồi sát cạnh một biểu tượng
const CAT_FRIEND_DISTANCE = 120;       // dừng gần bạn trong khoảng này thì quay sang và thả tim
const CAT_HEART_SECONDS = 1.8;
const CAT_VISIT_CHANCE = 0.25;         // tỉ lệ mèo chủ động đi tới ngồi cạnh một bạn mèo khác
const CAT_BODY_HALF = 44;              // nửa chiều dài thân mèo, dùng để né biểu tượng
const CAT_BODY_HEIGHT = 82;
const CAT_SIZE_RANGE = [72, 86];       // mèo to để nổi bật trên nền đồi
const CAT_GROUND_MARGIN = 26;          // khoảng né gốc lùm cây (mèo được phép đi phía trước/sau tán lá)
const CAT_GROOM_CHANCE = 0.45;         // khi không ngủ: tỉ lệ ngồi liếm lông thay vì ngồi ngắm
const CAT_SLEEP_POSES = ['loaf', 'side', 'back'];   // cuộn tròn, nằm nghiêng, nằm ngửa phơi bụng
const CAT_CUSHION_CHANCE = 0.35;       // tỉ lệ ngủ trên đệm hồng
const CAT_HAPPY_CHANCE = 0.65;         // tỉ lệ mèo có mắt cười híp "^ ^"
const CAT_COLLAR_CHANCE = 0.3;
const CAT_MARGIN_X = 70;
const CAT_MARGIN_BOTTOM = 60;           // chừa mép dưới cho lùm cây và mây khung viền
const CAT_HOP_SECONDS = 0.55;          // nhảy cẫng khi được vuốt ve
const CAT_LOVE_SECONDS = 1.6;          // chùm tim bay lên khi được vuốt ve
const CAT_JOY_SECONDS = 2.6;           // mắt cười híp sau khi được vuốt ve
const CAT_RUSH = 2.2;                  // tốc độ chạy tới cuộn len so với đi dạo
const CAT_PET_RADIUS = 46;
// Chỗ đứng quanh cuộn len (so với tâm len) cho tối đa 3 chú cùng vờn: trái, phải, phía sau.
const CAT_YARN_SLOTS = [[-40, -3], [40, -3], [-16, -20]];

// Biểu tượng chặn cả vùng thân mèo (mèo đứng dưới sẽ bị biểu tượng che);
// vật cản "ground" (gốc lùm cây) chỉ chặn chỗ đặt chân vì cây và mèo được vẽ xen kẽ theo chiều sâu.
function catBlockedAt(x, y, obstacles) {
  return obstacles.some(ob => ob.ground
    ? x > ob.x - CAT_GROUND_MARGIN && x < ob.x + ob.w + CAT_GROUND_MARGIN && y > ob.y - 8 && y < ob.y + ob.h + 8
    : x > ob.x - CAT_BODY_HALF && x < ob.x + ob.w + CAT_BODY_HALF && y > ob.y - 4 && y < ob.y + ob.h + CAT_BODY_HEIGHT);
}

function catPathClear(from, to, obstacles) {
  const steps = Math.ceil(Math.hypot(to.x - from.x, to.y - from.y) / 12);
  for (let i = 1; i <= steps; i++) {
    const t = i / steps;
    if (catBlockedAt(from.x + (to.x - from.x) * t, from.y + (to.y - from.y) * t, obstacles)) return false;
  }
  return true;
}

function catRandomSpot(world) {
  const rand = world.rand, others = world.cats || [];
  if (others.length > 1 && rand() < CAT_VISIT_CHANCE) {
    const friend = others[Math.floor(rand() * others.length)];
    return { x: friend.x + (rand() < 0.5 ? -1 : 1) * (95 + rand() * 20), y: friend.y + (rand() - 0.5) * 20 };
  }
  if (world.obstacles.length && rand() < CAT_SNUGGLE_CHANCE) {
    const ic = world.obstacles[Math.floor(rand() * world.obstacles.length)];
    const side = rand() < 0.5 ? -1 : 1;
    const x = side < 0 ? ic.x - CAT_BODY_HALF - 4 : ic.x + ic.w + CAT_BODY_HALF + 4;
    return { x, y: ic.y + ic.h * (0.55 + rand() * 0.4) };
  }
  return {
    x: CAT_MARGIN_X + rand() * (world.W - CAT_MARGIN_X * 2),
    y: world.top + rand() * (world.floor - world.top),
  };
}

function catInBounds(p, world) {
  return p.x >= CAT_MARGIN_X && p.x <= world.W - CAT_MARGIN_X && p.y >= world.top && p.y <= world.floor;
}

function catPickTarget(cat, world) {
  for (let tries = 0; tries < 40; tries++) {
    const spot = catRandomSpot(world);
    if (!catInBounds(spot, world) || catBlockedAt(spot.x, spot.y, world.obstacles)) continue;
    if (Math.hypot(spot.x - cat.x, spot.y - cat.y) < 60) continue;
    if (catPathClear(cat, spot, world.obstacles)) return spot;
  }
  return null;
}

function catFreeSpot(world) {
  for (let tries = 0; tries < 60; tries++) {
    const p = catRandomSpot(world);
    if (catInBounds(p, world) && !catBlockedAt(p.x, p.y, world.obstacles)) return p;
  }
  return null;
}

function createCat(world, coat) {
  const rand = world.rand;
  const spot = catFreeSpot(world) || { x: world.W / 2, y: world.floor };
  return {
    x: spot.x, y: spot.y, dir: rand() < 0.5 ? -1 : 1, coat,
    size: CAT_SIZE_RANGE[0] + rand() * (CAT_SIZE_RANGE[1] - CAT_SIZE_RANGE[0]), pace: 0.85 + rand() * 0.3,
    state: 'sit', timer: 1 + rand() * 4, target: null,
    phase: 0, clock: rand() * 10, sway: 0, blink: 0, blinkIn: 1 + rand() * 4, earTwitch: 0, heart: 0, sleepPose: 'loaf', cushion: false,
    hop: 0, love: 0, joy: 0, play: false, rush: 1, batIn: 0,
    mood: rand() < CAT_HAPPY_CHANCE ? 'happy' : 'open', collar: rand() < CAT_COLLAR_CHANCE,
  };
}

function catRest(cat, world, tune) {
  const rand = world.rand;
  const range = rand() < tune.naps ? CAT_SLEEP_SECONDS : CAT_SIT_SECONDS;
  const sleeping = range === CAT_SLEEP_SECONDS;
  cat.state = sleeping ? 'sleep' : rand() < CAT_GROOM_CHANCE ? 'groom' : 'sit';
  cat.rush = 1;
  if (cat.play) { cat.state = 'play'; cat.timer = YARN_SECONDS; cat.target = null; cat.batIn = 0.3; return; }
  if (sleeping) {
    cat.sleepPose = CAT_SLEEP_POSES[Math.floor(rand() * CAT_SLEEP_POSES.length)];
    cat.cushion = rand() < CAT_CUSHION_CHANCE;
  }
  cat.timer = range[0] + rand() * (range[1] - range[0]);
  cat.target = null;
  const friend = (world.cats || []).find(o => o !== cat && Math.hypot(o.x - cat.x, o.y - cat.y) < CAT_FRIEND_DISTANCE);
  if (friend) {
    cat.heart = CAT_HEART_SECONDS;
    cat.dir = friend.x >= cat.x ? 1 : -1;
  }
}

function catStartWalk(cat, world, tune) {
  const target = catPickTarget(cat, world);
  if (!target) { catRest(cat, world, { naps: 0 }); cat.timer = 2; return; }
  cat.state = 'walk';
  cat.target = target;
  if (tune.speed <= 0) catRest(cat, world, tune);
}

function catStepWalk(cat, dt, world, tune) {
  const dx = cat.target.x - cat.x, dy = cat.target.y - cat.y, dist = Math.hypot(dx, dy);
  const move = CAT_WALK_SPEED * Math.max(tune.speed, 0.3) * cat.pace * cat.rush * dt;
  if (Math.abs(dx) > 2) cat.dir = dx > 0 ? 1 : -1;
  if (dist <= move) {
    cat.x = cat.target.x; cat.y = cat.target.y;
    catRest(cat, world, tune);
    return;
  }
  cat.x += dx / dist * move; cat.y += dy / dist * move;
  cat.phase += move * 0.16;
}

function catStepFace(cat, dt, world) {
  cat.clock += dt;
  cat.sway = Math.sin(cat.clock * (cat.state === 'walk' ? 3.2 : 1.4));
  cat.blink = Math.max(0, cat.blink - dt);
  cat.heart = Math.max(0, cat.heart - dt);
  cat.hop = Math.max(0, cat.hop - dt);
  cat.love = Math.max(0, cat.love - dt);
  cat.joy = Math.max(0, cat.joy - dt);
  cat.blinkIn -= dt;
  if (cat.blinkIn <= 0) { cat.blink = 0.14; cat.blinkIn = 2 + world.rand() * 5; }
  cat.earTwitch = cat.state === 'sit' && (cat.clock % 7) < 0.25 ? 1 : 0;
}

// Sau khi biểu tượng di chuyển, mèo đang đứng/đi vào chỗ bị che sẽ chọn đường mới.
function catAdaptToIcons(cat, world, tune) {
  const blocked = catBlockedAt(cat.x, cat.y, world.obstacles);
  const pathBroken = cat.state === 'walk' && (!catInBounds(cat.target, world) || !catPathClear(cat, cat.target, world.obstacles));
  const outside = !catInBounds(cat, world);
  if (!blocked && !pathBroken && !outside) return;
  // Biểu tượng vừa được thả đè lên mèo: đặt mèo sang một chỗ trống khác thay vì để bị che.
  if (blocked || outside) {
    const spot = catFreeSpot(world);
    if (spot) { cat.x = spot.x; cat.y = spot.y; }
  }
  catStartWalk(cat, world, tune);
}

function stepCat(cat, dt, world, tune) {
  catStepFace(cat, dt, world);
  if (cat.state === 'walk') { catStepWalk(cat, dt, world, tune); return; }
  cat.timer -= dt;
  if (cat.timer <= 0) catStartWalk(cat, world, tune);
}

// ---------------------------------------------------------------- tương tác chuột (cats-interact.js gọi)
// Mèo dưới con trỏ: ưu tiên chú gần nhất trong vùng thân (từ chân lên tới tai).
function catAt(cats, x, y) {
  let best = null, bestD = Infinity;
  for (const cat of cats) {
    const k = cat.size / 60, d = Math.hypot(cat.x - x, cat.y - 32 * k - y);
    if (d < CAT_PET_RADIUS * k && d < bestD) { best = cat; bestD = d; }
  }
  return best;
}

// Vuốt ve: mèo nhảy cẫng, thả chùm tim, cười híp mắt và ngồi lại một lúc (mèo đang ngủ thì tỉnh dậy).
function catPet(cat, x) {
  cat.hop = CAT_HOP_SECONDS; cat.love = CAT_LOVE_SECONDS; cat.joy = CAT_JOY_SECONDS; cat.blink = 0;
  cat.dir = x >= cat.x ? 1 : -1;
  if (cat.state === 'sleep' || cat.state === 'walk') { cat.state = 'sit'; cat.target = null; cat.play = false; cat.rush = 1; }
  if (cat.state !== 'play') cat.timer = 2.5 + Math.random() * 2;
}

// Chạy tới chỗ đứng `slot` cạnh cuộn len rồi chuyển sang vờn khi tới nơi.
function catChase(cat, yarn, world, slot) {
  const [dx, dy] = CAT_YARN_SLOTS[slot % CAT_YARN_SLOTS.length], spot = { x: yarn.x + dx, y: yarn.y + dy };
  if (catBlockedAt(spot.x, spot.y, world.obstacles)) return;
  cat.play = true; cat.slot = slot; cat.rush = CAT_RUSH; cat.state = 'walk'; cat.target = spot;
}
