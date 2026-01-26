public final class Input {
    private Input() {}

    public static void helper() {}
}

final class Singleton {
    private Singleton() {}

    public static Singleton getInstance() {
        return null;
    }
}

final class MultiplePrivate {
    private MultiplePrivate() {}
    private MultiplePrivate(int x) {}
}
