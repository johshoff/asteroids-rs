@0xae800a512bc1699b;

struct GameStatus {
  timestamp @0 : UInt64;   # milliseconds since server-defined epoch
  ships     @1 : List(Ship);
}

struct Ship {
  id     @0 : UInt8;
  x      @1 : Float32;
  dx     @2 : Float32;
  y      @3 : Float32;
  dy     @4 : Float32;
  ang    @5 : Float32;
  dang   @6 : Float32;
}

struct ShipInfo {
  id     @0 : UInt8;
  name   @1 : Text;
  r      @2 : UInt8;
  g      @3 : UInt8;
  b      @4 : UInt8;
}

struct PlayerStatus {
  throttle  @0 : Bool;
  turnLeft  @1 : Bool;
  turnRight @2 : Bool;
}

