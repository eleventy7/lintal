public class Input {
    private Input() {}

    public static void helper() {}
}

class Singleton {
    private Singleton() {}

    public static Singleton getInstance() {
        return null;
    }
}

class MultiplePrivate {
    private MultiplePrivate() {}
    private MultiplePrivate(int x) {}
}
