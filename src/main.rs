use hamster::hamt::HAMT;



pub fn main() {

  let mut a = HAMT::new();
  for k in 1..7 {
    a = a.set(k, - (k as i32));
    // for j in 1..k {
    //   if *a.get(j).unwrap() != -(j as i32) {
    //     println!("Here! it {}, item {}", k, j);
    //   }
    // }
  }
  for k in 1..7 {
    println!("key: {} value: {}", k, a.get(k).unwrap());
  }
  println!("yike");
}
