

---@class Transport @parent class
---@public field name string
local transport = {}

function transport:move()end

---@class Car : Transport @Car extends Transport
local car = {}
function car:move()end

---@class Ship : Transport @Ship extends Transport
local ship = {}

---@param type number @parameter type
---@return Car|Ship @may return Car or Ship
local function create(type)
-- ignored
end

local obj = create(1)
---now you can see completion for obj

---@type Car
local obj2
---now you can see completion for obj2

local list = { obj, obj2 }
---@param v Transport
for _, v in ipairs(list) do
---not you can see completion for v
end
