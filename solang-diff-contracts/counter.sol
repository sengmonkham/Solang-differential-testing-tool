contract counter {
  unit64 public count = 0;

  function increment() public return (unit64) {
    count += 1;
    return count;
  }

  function get() public view returns (unit64) {
    return count;
  }
}
