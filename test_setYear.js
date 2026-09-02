let d = new Date(2000, 0, 1);
console.log(d.getYear());
console.log(d.setYear);
if (d.setYear) {
    d.setYear(110);
    console.log(d.getYear());
} else {
    console.log("setYear is undefined");
}
