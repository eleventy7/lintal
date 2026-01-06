public class Input {
    String dayName(int day) {
        return switch (day) {
            case 1 -> {
                yield "Monday";
            }
            case 2 -> {
                yield "Tuesday";
            }
            case 3 -> {
                yield "Wednesday";
            }
            case 4 -> {
                yield "Thursday";
            }
            case 5 -> {
                yield "Friday";
            }
            default -> {
                yield "Weekend";
            }
        };
    }

    int calculate(String op, int a, int b) {
        return switch (op) {
            case "add" -> {
                int result = a + b;
                yield result;
            }
            case "sub" -> {
                int result = a - b;
                yield result;
            }
            case "mul" -> {
                int result = a * b;
                yield result;
            }
            default -> {
                yield 0;
            }
        };
    }

    void switchStatement(int x) {
        switch (x) {
            case 1:
                System.out.println("one");
                break;
            case 2:
                System.out.println("two");
                break;
            case 3:
                System.out.println("three");
                break;
            default:
                System.out.println("other");
        }
    }

    String nestedSwitch(int x, int y) {
        return switch (x) {
            case 1 -> {
                String inner = switch (y) {
                    case 1 -> {
                        yield "one-one";
                    }
                    case 2 -> {
                        yield "one-two";
                    }
                    default -> {
                        yield "one-other";
                    }
                };
                yield inner;
            }
            default -> {
                yield "other";
            }
        };
    }
}
