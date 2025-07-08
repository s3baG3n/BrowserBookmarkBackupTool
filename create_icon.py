# Python script to create icon.ico file for the project
# Run this to generate the missing icon.ico file

from PIL import Image, ImageDraw
import os

def create_backup_icon():
    # Create a 256x256 image with transparent background
    sizes = [(16, 16), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)]
    images = []
    
    for size in sizes:
        # Create image with transparent background
        img = Image.new('RGBA', size, (0, 0, 0, 0))
        draw = ImageDraw.Draw(img)
        
        # Scale factor
        scale = size[0] / 256
        
        # Draw folder (blue)
        folder_color = (33, 150, 243, 255)  # Material Blue
        folder_left = int(20 * scale)
        folder_top = int(60 * scale)
        folder_right = int(236 * scale)
        folder_bottom = int(200 * scale)
        
        # Folder tab
        draw.rectangle([folder_left, folder_top, int(100 * scale), int(80 * scale)], 
                      fill=folder_color)
        # Folder body
        draw.rectangle([folder_left, int(80 * scale), folder_right, folder_bottom], 
                      fill=folder_color)
        
        # Draw arrow (green) pointing upward
        arrow_color = (76, 175, 80, 255)  # Material Green
        arrow_width = int(40 * scale)
        arrow_height = int(80 * scale)
        arrow_x = int(128 * scale)
        arrow_y = int(100 * scale)
        
        # Arrow shaft
        shaft_left = arrow_x - int(arrow_width // 4)
        shaft_right = arrow_x + int(arrow_width // 4)
        shaft_top = arrow_y
        shaft_bottom = arrow_y + int(arrow_height * 0.6)
        draw.rectangle([shaft_left, shaft_top, shaft_right, shaft_bottom], 
                      fill=arrow_color)
        
        # Arrow head (triangle)
        head_width = int(arrow_width * 0.8)
        head_height = int(arrow_height * 0.4)
        head_top = arrow_y - int(head_height // 2)
        
        arrow_points = [
            (arrow_x, head_top),  # Top point
            (arrow_x - head_width // 2, arrow_y + head_height // 2),  # Bottom left
            (arrow_x + head_width // 2, arrow_y + head_height // 2),  # Bottom right
        ]
        draw.polygon(arrow_points, fill=arrow_color)
        
        images.append(img)
    
    # Save as ICO
    images[0].save('icon.ico', format='ICO', sizes=[(img.width, img.height) for img in images], 
                   append_images=images[1:])
    print("Created icon.ico successfully!")

if __name__ == "__main__":
    try:
        create_backup_icon()
    except ImportError:
        print("Please install Pillow first: pip install Pillow")
        print("\nAlternatively, you can download a free icon from:")
        print("- https://www.iconfinder.com/search?q=backup+folder")
        print("- https://www.flaticon.com/search?word=backup")
        print("- https://icons8.com/icons/set/backup")
        print("\nMake sure to save it as 'icon.ico' in the project root.")