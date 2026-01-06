public class Input {
    void ifElseChain(int x) {
        if (x > 0) {
            System.out.println("positive");
        } else if (x < 0) {
            System.out.println("negative");
        } else {
            System.out.println("zero");
        }
    }

    void nestedIf(int x, int y) {
        if (x > 0) {
            if (y > 0) {
                System.out.println("both positive");
                if (x > y) {
                    System.out.println("x greater");
                }
            }
        }
    }

    void forLoop() {
        for (int i = 0; i < 10; i++) {
            System.out.println(i);
            for (int j = 0; j < 5; j++) {
                System.out.println(j);
            }
        }
    }

    void whileLoop(int n) {
        while (n > 0) {
            System.out.println(n);
            n--;
        }
    }

    void doWhileLoop(int n) {
        do {
            System.out.println(n);
            n--;
        } while (n > 0);
    }

    void enhancedFor(String[] args) {
        for (String arg : args) {
            System.out.println(arg);
            if (arg.isEmpty()) {
                continue;
            }
        }
    }

    void switchStatement(int day) {
        switch (day) {
            case 1:
                System.out.println("Monday");
                break;
            case 2:
                System.out.println("Tuesday");
                break;
            default:
                System.out.println("Other");
        }
    }
}
