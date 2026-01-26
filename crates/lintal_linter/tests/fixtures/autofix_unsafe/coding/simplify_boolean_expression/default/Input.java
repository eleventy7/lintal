public class Input {
    public void method(boolean flag) {
        boolean a = flag == true;
        boolean b = flag == false;
        boolean c = flag != true;
        boolean d = flag != false;
        boolean e = !true;
        boolean f = !false;
        boolean g = flag ? true : false;
        boolean h = flag ? false : true;
        System.out.println("" + a + b + c + d + e + f + g + h);
    }
}
